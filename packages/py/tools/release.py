#!/usr/bin/env python3
"""Plan, preserve, verify and finalize independent releases. Never publish packages."""
from __future__ import annotations

import argparse
import base64
from dataclasses import dataclass
from email.parser import BytesParser
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode, urlsplit
from urllib.request import Request, build_opener, HTTPRedirectHandler
import zipfile

from processes import install_signal_handlers, run as run_process
from repo_paths import REPO_ROOT

ROOT = REPO_ROOT
REPOSITORY = "Anbani/anbani"
CRATES = ("anbani-core", "anbani-api", "anbani-spell", "anbani-tui", "anbani-mcp", "anbani-cli")
PLATFORMS = {
    "linux-x64-gnu": ("linux", "x64"), "linux-arm64-gnu": ("linux", "arm64"),
    "darwin-x64": ("darwin", "x64"), "darwin-arm64": ("darwin", "arm64"),
    "win32-x64-msvc": ("win32", "x64"),
}
WHEEL_PLATFORMS = {"manylinux_2_28_x86_64", "manylinux_2_28_aarch64", "macosx_x86_64", "macosx_arm64", "win_amd64"}
MANIFEST = "release-manifest.json"
CHECKSUMS = "SHA256SUMS"
MAX_DOWNLOAD = 512 * 1024 * 1024
SEMVER = r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"

class ReleaseError(Exception):
    pass

def require(condition, message):
    if not condition:
        raise ReleaseError(message)

def parse_tag(tag):
    match = re.fullmatch(rf"(js|py|textart|rs)-v({SEMVER})", tag)
    require(match is not None, "tag must be <js|py|textart|rs>-v<strict semver>")
    family, version = match.groups()
    pre = version.split("+", 1)[0].partition("-")[2]
    require(not any(p.isdigit() and len(p) > 1 and p.startswith("0") for p in pre.split(".")), "numeric prerelease identifiers cannot have leading zeros")
    return family, version

def python_version(value):
    require(isinstance(value, str), "Python version must be a string")
    match = re.fullmatch(
        r"([0-9]+)\.([0-9]+)\.([0-9]+)"
        r"(?:[-_.]?(alpha|beta|preview|pre|rc|a|b|c)[-_.]?([0-9]+)?)?"
        r"(?:[-_.]?(dev)[-_.]?([0-9]+)?)?"
        r"(?:\+([a-z0-9]+(?:[-_.][a-z0-9]+)*))?",
        value, re.IGNORECASE | re.ASCII,
    )
    require(match is not None, "version cannot be represented as a matching Python prerelease")
    major, minor, patch, phase, number, dev, dev_number, local = match.groups()
    normalized = ".".join(str(int(n)) for n in (major, minor, patch))
    if phase:
        phase = {"alpha": "a", "beta": "b", "c": "rc", "pre": "rc", "preview": "rc"}.get(phase.lower(), phase.lower())
        normalized += phase + str(int(number or "0"))
    if dev:
        normalized += ".dev" + str(int(dev_number or "0"))
    if local:
        normalized += "+" + ".".join(str(int(p)) if p.isdigit() else p.lower() for p in re.split(r"[-_.]", local))
    return normalized

def pypi_version(value):
    version = python_version(value)
    require("+" not in version, "PyPI does not accept local versions; choose a release tag without build metadata")
    return version

def run(root, arguments, timeout=30):
    try:
        result = run_process(arguments, cwd=root, capture_output=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ReleaseError(f"{arguments[0]} failed or timed out") from error
    require(result.returncode == 0, f"{arguments[0]} failed (exit {result.returncode})")
    return result.stdout

def git(root, *arguments):
    return run(root, ["git", *arguments]).decode("utf-8").strip()

def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode()

def json_read(data, label):
    try:
        return json.loads(data)
    except (ValueError, UnicodeError) as error:
        raise ReleaseError(f"invalid JSON: {label}") from error

class SafeRedirects(HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, newurl):
        require(urlsplit(newurl).scheme == "https", "refusing non-HTTPS redirect")
        redirected = super().redirect_request(request, fp, code, msg, headers, newurl)
        if redirected and urlsplit(newurl).netloc != urlsplit(request.full_url).netloc:
            redirected.remove_header("Authorization")
        return redirected

class HTTP:
    def request(self, method, url, *, headers=None, data=None, limit=MAX_DOWNLOAD):
        require(urlsplit(url).scheme == "https", "only HTTPS endpoints are permitted")
        request = Request(url, data=data, headers={"User-Agent": "anbani-release/1", **(headers or {})}, method=method)
        try:
            with build_opener(SafeRedirects()).open(request, timeout=30) as response:
                body = response.read(limit + 1)
                require(len(body) <= limit, "HTTP response exceeds size limit")
                return response.status, dict(response.headers), body
        except HTTPError as error:
            return error.code, dict(error.headers), b""
        except (URLError, OSError, TimeoutError) as error:
            raise ReleaseError("HTTP connection failed or timed out") from error

class GitHub:
    def __init__(self, http=None, token=None):
        self.http = http or HTTP()
        self.token = token if token is not None else os.environ.get("GH_TOKEN", os.environ.get("GITHUB_TOKEN", ""))
        require(bool(self.token), "GH_TOKEN or GITHUB_TOKEN is required for authenticated release safety reads")

    def request(self, method, path, payload=None, *, raw=False, data=None):
        url = path if path.startswith("https://") else "https://api.github.com" + path
        host = urlsplit(url).netloc
        require(host in {"api.github.com", "uploads.github.com"}, "invalid authenticated GitHub endpoint")
        require(urlsplit(url).path.startswith(f"/repos/{REPOSITORY}/"), "GitHub endpoint must belong to Anbani/anbani")
        headers = {"Authorization": "Bearer " + self.token, "Accept": "application/octet-stream" if raw else "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28"}
        if payload is not None:
            data = json_bytes(payload)
            headers["Content-Type"] = "application/json"
        elif data is not None:
            headers["Content-Type"] = "application/octet-stream"
        status, _, body = self.http.request(method, url, headers=headers, data=data)
        expected = {"GET": {200}, "POST": {201}, "PATCH": {200}, "DELETE": {204}}[method]
        require(status in expected, f"GitHub {method} failed with HTTP {status}")
        return body if raw else (json_read(body, "GitHub response") if body else None)

    def pages(self, path, key=None):
        result = []
        for page in range(1, 101):
            separator = "&" if "?" in path else "?"
            data = self.request("GET", path + separator + urlencode({"per_page": 100, "page": page}))
            rows = data[key] if key else data
            require(isinstance(rows, list), "GitHub pagination must return a list")
            result.extend(rows)
            if len(rows) < 100:
                return result
        raise ReleaseError("GitHub pagination exceeded 100 pages")

    def existing_tag(self, root, tag):
        remote = self.request("GET", f"/repos/{REPOSITORY}/git/ref/tags/{quote(tag, safe='')}")
        local = git(root, "rev-parse", "--verify", f"refs/tags/{tag}")
        require(remote.get("ref") == "refs/tags/" + tag and remote.get("object", {}).get("sha") == local, "remote release tag differs from local tag object")

    def safety(self, root, sha, tag=None):
        prefix = f"/repos/{REPOSITORY}"
        if tag is not None:
            self.existing_tag(root, tag)
        main = self.request("GET", prefix + "/commits/main")["sha"]
        require(re.fullmatch(r"[0-9a-f]{40}", main) is not None, "invalid remote main SHA")
        git(root, "merge-base", "--is-ancestor", sha, main)
        runs = self.pages(prefix + "/actions/workflows/ci.yml/runs?" + urlencode({"head_sha": sha}), "workflow_runs")
        valid_runs = [r for r in runs if r.get("head_sha") == sha and r.get("event") in {"push", "pull_request"} and r.get("status") == "completed" and r.get("conclusion") == "success"]
        require(valid_runs, "no successful exact-SHA ci.yml run")
        checks = self.pages(prefix + f"/commits/{sha}/check-runs", "check_runs")
        suites = {r.get("check_suite_id") for r in valid_runs}
        require(any(c.get("name") == "ci-gate" and c.get("head_sha") == sha and c.get("status") == "completed" and c.get("conclusion") == "success" and c.get("app", {}).get("slug") == "github-actions" and c.get("check_suite", {}).get("id") in suites for c in checks), "no successful exact-SHA ci-gate from ci.yml")

    def release(self, tag):
        matches = [r for r in self.pages(f"/repos/{REPOSITORY}/releases") if r.get("tag_name") == tag]
        require(len(matches) <= 1, "multiple GitHub releases for tag")
        return matches[0] if matches else None

    def assets(self, release_id):
        return self.pages(f"/repos/{REPOSITORY}/releases/{release_id}/assets")

    def download(self, asset_id):
        return self.request("GET", f"/repos/{REPOSITORY}/releases/assets/{asset_id}", raw=True)

@dataclass
class Context:
    root: Path
    tag: str
    family: str
    version: str
    source_sha: str
    publication_holds: list
    api: object = None

def hold_records(document):
    rows = document.get("publication_holds", [])
    require(isinstance(rows, list), "publication_holds must be a list")
    require(all(isinstance(r, dict) and r.get("status") in {"open", "resolved", "closed", "cleared"} for r in rows), "publication hold has invalid status")
    return [r for r in rows if r["status"] == "open"]

def load_context(root, tag, api=None, *, check_remote=True):
    root = Path(root).resolve()
    family, version = parse_tag(tag)
    if family in {"py", "rs"}:
        pypi_version(version)
    if family in {"js", "textart"}:
        require("+" not in version, "npm strips SemVer build metadata; choose a tag without build metadata")
    sha = git(root, "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}")
    require(re.fullmatch(r"[0-9a-f]{40}", sha) is not None, "tag does not resolve to a commit")

    def source(path):
        return run(root, ["git", "show", f"{sha}:{path}"])

    def js(path):
        return json_read(source(path), path)

    def toml(path):
        return tomllib.loads(source(path).decode())

    if family in {"js", "textart"}:
        document = js({"js": "packages/js/anbani/package.json", "textart": "packages/js/textart/package.json"}[family])
        require(document.get("name") == {"js": "anbani", "textart": "anbani-textart"}[family], "unexpected npm package name")
        require(document.get("version") == version, "tag version differs from package manifest")
    elif family == "py":
        project = toml("packages/py/anbani/pyproject.toml")["project"]
        require(project.get("name") == "anbani" and python_version(project.get("version")) == pypi_version(version), "tag version differs from Python manifest")
    else:
        cargo = toml("packages/rs/Cargo.toml")
        require(cargo["workspace"]["package"]["version"] == version, "tag version differs from Rust workspace")
        native = js("packages/js/native/package.json")
        require(native.get("name") == "@anbani/native" and native.get("version") == version and native.get("private") is True and "optionalDependencies" not in native, "native build descriptor differs from workspace")
        members = cargo["workspace"]["members"]
        documents = {path: toml("packages/rs/" + path + "/Cargo.toml") for path in members}
        names = {d["package"]["name"] for d in documents.values()}
        require(set(CRATES) <= names, "missing publishable workspace crates")
        for document in documents.values():
            package = document["package"]
            require(package.get("version") == {"workspace": True}, "all Rust members must inherit workspace version")
            require((package.get("publish") is not False) == (package["name"] in CRATES), "unexpected publishable crate")

        def pins(document):
            for key, value in document.items():
                if isinstance(value, dict):
                    if key in names and ("path" in value or "version" in value):
                        require(value.get("version") == version, f"workspace dependency pin differs: {key}")
                    pins(value)
        pins(cargo)
        for document in documents.values():
            pins(document)
        lock = toml("packages/rs/Cargo.lock")
        require({p["name"] for p in lock["package"] if "source" not in p} == names, "Cargo.lock does not contain exactly the workspace packages")
        for package in lock["package"]:
            if package["name"] in names and "source" not in package:
                require(package["version"] == version, "Cargo.lock workspace version differs")
        channel = toml("packages/rs/rust-toolchain.toml")["toolchain"]["channel"]
        require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", channel) is not None, "release requires an exact x.y.z Rust channel in rust-toolchain.toml")
    holds = hold_records(js("common/data/manifest.json"))
    current = root / "common/data/manifest.json"
    if current.exists():
        for record in hold_records(json_read(current.read_bytes(), str(current))):
            if record not in holds:
                holds.append(record)
    if check_remote:
        api = api or GitHub()
        api.safety(root, sha, tag)
    else:
        git(root, "merge-base", "--is-ancestor", sha, "refs/remotes/origin/main")
        require(git(root, "rev-parse", "HEAD") == sha, "verify requires exact tagged checkout")
    return Context(root, tag, family, version, sha, holds, api)

def require_unheld(context):
    require(not context.publication_holds, "publication is held by common/data/manifest.json; no mutation performed")

def family_items(context):
    family, version = context.family, context.version
    if family in {"js", "textart"}:
        packages = [("npm", "anbani" if family == "js" else "anbani-textart")]
    elif family == "py":
        packages = [("pypi", "anbani")]
    else:
        packages = [("crates.io", name) for name in CRATES]
        packages += [("npm", "@anbani/native-" + key) for key in PLATFORMS]
        packages += [("npm", "@anbani/native"), ("npm", "@anbani/wasm"), ("pypi", "anbani-rs")]
        packages += [("github", name) for name in ("cli-x86_64", "cli-aarch64", "wasm", "c-header")]
    return [{"registry": registry, "package": package, "version": pypi_version(version) if registry == "pypi" else version} for registry, package in packages]

class Registry:
    def __init__(self, http=None):
        self.http = http or HTTP()
        self.documents = {}
        self.identities = {}
        self.listings = {}

    def get(self, url):
        status, _, body = self.http.request("GET", url, limit=8 * 1024 * 1024)
        require(status in {200, 404}, f"registry probe failed with HTTP {status}")
        return None if status == 404 else json_read(body, "registry response")

    def probe(self, item):
        registry, name, version = item["registry"], item["package"], item["version"]
        key = (registry, name, version)
        if key in self.documents:
            return "present" if self.documents[key] is not None else "missing"
        if registry == "npm":
            base = "https://registry.npmjs.org/" + quote(name, safe="")
            identity = self.get(base)
            document = self.get(base + "/" + quote(version, safe=""))
            if document is not None:
                require(document.get("name") == name and document.get("version") == version, "npm returned wrong package/version")
        elif registry == "pypi":
            identity = True
            document = self.get(f"https://pypi.org/pypi/{quote(name)}/{quote(version)}/json")
            if document is not None:
                info = document.get("info", {})
                require(normalize_name(info.get("name", "")) == normalize_name(name) and python_version(info.get("version")) == version, "PyPI returned wrong package/version")
                require(isinstance(document.get("urls"), list), "PyPI response has no file list")
        elif registry == "crates.io":
            base = "https://crates.io/api/v1/crates/" + quote(name)
            identity = self.get(base)
            response = self.get(base + "/" + quote(version))
            document = None if response is None else response.get("version")
            if response is not None:
                require(isinstance(document, dict) and document.get("crate") == name and document.get("num") == version, "crates.io returned wrong package/version")
        else:
            require(registry == "github", "unknown registry")
            identity, document = True, None
        require(document is None or identity is not None, "registry version exists but package identity is absent")
        self.identities[(registry, name)] = identity is not None
        if registry in {"npm", "crates.io"}:
            self.listings[(registry, name)] = published_versions(registry, identity)
        self.documents[key] = document
        return "present" if document is not None else "missing"

    def require_newer(self, item):
        registry, name, version = item["registry"], item["package"], item["version"]
        key = (registry, name)
        if registry == "pypi" and key not in self.listings:
            self.listings[key] = published_versions(registry, self.get(f"https://pypi.org/pypi/{quote(name)}/json"))
        newest = max(self.listings.get(key, []), key=version_key, default=None)
        require(newest is None or version_key(version) > version_key(newest), f"{registry} {name} {version} does not outrank published {newest}; choose a higher version")

    def file_state(self, item, file):
        self.probe(item)
        document = self.documents[(item["registry"], item["package"], item["version"])]
        if document is None:
            return "missing"
        registry = item["registry"]
        if registry == "pypi":
            matches = [row for row in document["urls"] if row.get("filename") == file["name"]]
            require(len(matches) <= 1, "duplicate PyPI filename")
            if not matches:
                return "missing"
            actual = matches[0].get("digests", {}).get("sha256")
        elif registry == "crates.io":
            actual = document.get("checksum")
        elif registry == "npm":
            dist = document.get("dist", {})
            url = dist.get("tarball", "")
            require(urlsplit(url).scheme == "https" and urlsplit(url).netloc == "registry.npmjs.org", "invalid npm tarball URL")
            status, _, body = self.http.request("GET", url)
            require(status == 200, f"npm tarball fetch failed with HTTP {status}")
            integrity = dist.get("integrity")
            if integrity:
                candidates = integrity.split()
                require(any(token == "sha512-" + base64.b64encode(hashlib.sha512(body).digest()).decode() for token in candidates), "npm registry tarball integrity mismatch")
            else:
                require(dist.get("shasum") == hashlib.sha1(body).hexdigest(), "npm registry tarball shasum mismatch")
            actual = hashlib.sha256(body).hexdigest()
        else:
            return "missing"
        require(isinstance(actual, str) and re.fullmatch(r"[0-9a-f]{64}", actual) is not None, "registry lacks valid SHA256 digest")
        require(actual == file["sha256"], f"registry digest conflict: {file['name']}; use a new patch version")
        return "present"

def version_key(value):
    match = re.match(r"([0-9]+)\.([0-9]+)\.([0-9]+)(.*)", value or "")
    if match is None:
        return (-1, -1, -1, 0, "")
    major, minor, patch, rest = match.groups()
    rest = rest.split("+", 1)[0]
    return (int(major), int(minor), int(patch), 0 if rest else 1, rest)

def published_versions(registry, document):
    if not document:
        return []
    if registry == "npm":
        versions = list(document.get("versions", {}))
    elif registry == "crates.io":
        versions = [row.get("num") for row in document.get("versions", []) if isinstance(row, dict)]
    else:
        versions = list(document.get("releases", {}))
    return [v for v in versions if isinstance(v, str)]

def selection(context, registry):
    items = family_items(context)
    for item in items:
        item["status"] = registry.probe(item)
    for item in items:
        if item["status"] == "missing" and item["registry"] != "github":
            registry.require_newer(item)
    bootstrap = [{"registry": i["registry"], "package": i["package"]} for i in items if i["registry"] in {"npm", "crates.io"} and not registry.identities[(i["registry"], i["package"])]]
    return {"family": context.family, "version": context.version, "source_sha": context.source_sha, "items": items, "bootstrap_required": bootstrap, "publication_holds": context.publication_holds}

def plan(context, registry=None):
    result = selection(context, registry or Registry())
    assets = [i for i in result["items"] if i["registry"] == "github"]
    if assets:
        record = context.api.release(context.tag)
        if record is not None:
            names = {a["name"] for a in context.api.assets(record["id"])}
            version = context.version
            expected = {
                "cli-x86_64": f"anbani-v{version}-x86_64-linux-musl.tar.gz",
                "cli-aarch64": f"anbani-v{version}-aarch64-linux-musl.tar.gz",
                "wasm": f"anbani-wasm-v{version}.tar.gz",
                "c-header": f"anbani-c-header-v{version}.tar.gz",
            }
            for item in assets:
                item["status"] = "present" if expected[item["package"]] in names else "missing"
    return result

def normalize_name(name):
    return re.sub(r"[-_.]+", "-", name).lower()

def archive_members(path):
    try:
        with tarfile.open(path, "r:gz") as archive:
            members = archive.getmembers()
            require(len(members) <= 100000, "archive has too many members")
            require(len({m.name for m in members}) == len(members), "archive has duplicate members")
            require(sum(m.size for m in members) <= MAX_DOWNLOAD, "expanded archive exceeds size limit")
            require(all(m.isfile() or m.isdir() for m in members), "archive contains links or special files")
            for member in members:
                require(not PurePosixPath(member.name).is_absolute() and ".." not in PurePosixPath(member.name).parts, "unsafe archive member path")
            return {m.name: archive.extractfile(m).read() for m in members if m.isfile() and (m.name.endswith(("package.json", "Cargo.toml", "PKG-INFO", "WHEEL")) or m.name.rsplit("/", 1)[-1] in {"LICENSE", "NOTICE"}) and m.size <= 2 * 1024 * 1024}
    except (tarfile.TarError, OSError) as error:
        raise ReleaseError(f"invalid archive: {path.name}") from error

def wheel_platform(filename):
    match = re.fullmatch(r"anbani_rs-[^-]+-cp39-abi3-(.+)\.whl", filename)
    require(match is not None, "Rust wheel must be cp39-abi3")
    platforms = match[1].split(".")
    if "manylinux_2_28_x86_64" in platforms:
        return "manylinux_2_28_x86_64"
    if "manylinux_2_28_aarch64" in platforms:
        return "manylinux_2_28_aarch64"
    if any(re.fullmatch(r"macosx_\d+_\d+_x86_64", p) for p in platforms):
        return "macosx_x86_64"
    if any(re.fullmatch(r"macosx_\d+_\d+_arm64", p) for p in platforms):
        return "macosx_arm64"
    require(platforms == ["win_amd64"], "unsupported wheel platform")
    return "win_amd64"

def payload_identity(context, path):
    version = context.version
    if path.name.endswith(".whl"):
        try:
            with zipfile.ZipFile(path) as archive:
                names = archive.namelist()
                require(len(names) == len(set(names)) and len(names) <= 100000, "wheel has duplicate or excessive members")
                require(all(not PurePosixPath(n).is_absolute() and ".." not in PurePosixPath(n).parts for n in names), "unsafe wheel member path")
                require(sum(i.file_size for i in archive.infolist()) <= MAX_DOWNLOAD, "expanded wheel exceeds size limit")
                metadata = [n for n in names if n.endswith(".dist-info/METADATA")]
                require(len(metadata) == 1, "wheel must contain exactly one METADATA")
                require(archive.getinfo(metadata[0]).file_size <= 2 * 1024 * 1024, "oversized wheel metadata")
                info = BytesParser().parsebytes(archive.read(metadata[0]))
                package = normalize_name(info["Name"] or "")
                python = pypi_version(version)
                require(python_version(info["Version"]) == python, "wheel version mismatch")
                require(path.name.startswith(package.replace("-", "_") + "-" + python + "-"), "wheel filename version/name differs from METADATA")
                if context.family == "rs":
                    require(package == "anbani-rs", "wrong binding wheel name")
                    wheel = metadata[0].rsplit("/", 1)[0] + "/WHEEL"
                    require(archive.getinfo(wheel).file_size <= 16384, "oversized WHEEL metadata")
                    tags = BytesParser().parsebytes(archive.read(wheel)).get_all("Tag", [])
                    require(tags and all(t.startswith("cp39-abi3-") for t in tags), "wheel metadata is not cp39-abi3")
                    role = wheel_platform(path.name)
                    require(set(t.split("-", 2)[2] for t in tags) == set(path.name[:-4].rsplit("-", 1)[1].split(".")), "wheel filename and WHEEL tags disagree")
                else:
                    require(context.family == "py" and package == "anbani", "unexpected wheel")
                    require(path.name.endswith("-py3-none-any.whl"), "pure Python wheel must be py3-none-any")
                    role = "wheel"
                return "pypi", package, role
        except (zipfile.BadZipFile, KeyError, OSError) as error:
            raise ReleaseError(f"invalid wheel: {path.name}") from error
    members = archive_members(path)
    if path.name.endswith(".tgz"):
        require("package/package.json" in members, "npm archive lacks package metadata")
        metadata = json_read(members["package/package.json"], path.name)
        package = metadata.get("name")
        require(metadata.get("version") == version and metadata.get("private") is not True, "npm archive version/private mismatch")
        if package == "@anbani/native":
            require(metadata.get("optionalDependencies") == {"@anbani/native-" + k: version for k in PLATFORMS}, "native loader must depend on all five exact-version platforms")
            require(metadata.get("main") == "index.cjs", "native loader main mismatch")
        elif isinstance(package, str) and package.startswith("@anbani/native-"):
            key = package[len("@anbani/native-"):]
            require(key in PLATFORMS, "unsupported native package")
            os_name, cpu = PLATFORMS[key]
            require(metadata.get("os") == [os_name] and metadata.get("cpu") == [cpu] and metadata.get("main") == "anbani.node", "native platform metadata mismatch")
            if os_name == "linux":
                require(metadata.get("libc") == ["glibc"], "GNU native package must declare glibc")
            with tarfile.open(path, "r:gz") as archive:
                binaries = [m for m in archive.getmembers() if m.isfile() and m.name.endswith((".node", ".so", ".dll", ".dylib"))]
                require(len(binaries) == 1 and binaries[0].name == "package/anbani.node" and binaries[0].size > 64, "native package needs exactly one nonempty target binary")
                with archive.extractfile(binaries[0]) as binary:
                    header = binary.read(64)
                    if os_name == "linux":
                        require(header[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", header, 18)[0] == (62 if cpu == "x64" else 183), "native ELF architecture mismatch")
                    elif os_name == "darwin":
                        require(header[:4] == b"\xcf\xfa\xed\xfe" and struct.unpack_from("<I", header, 4)[0] == (0x01000007 if cpu == "x64" else 0x0100000C), "native Mach-O architecture mismatch")
                    else:
                        require(header[:2] == b"MZ", "native Windows binary lacks PE header")
                        offset = struct.unpack_from("<I", header, 60)[0]
                        require(64 <= offset <= binaries[0].size - 6, "invalid PE header offset")
                        binary.seek(offset)
                        pe = binary.read(6)
                        require(pe[:4] == b"PE\0\0" and struct.unpack_from("<H", pe, 4)[0] == 0x8664, "native PE architecture mismatch")
        return "npm", package, "npm"
    if path.name.endswith(".crate"):
        metadata = [body for name, body in members.items() if name.count("/") == 1 and name.endswith("/Cargo.toml")]
        require(len(metadata) == 1, "crate archive lacks unique normalized Cargo.toml")
        package = tomllib.loads(metadata[0].decode())["package"]
        require(package.get("version") == version and path.name == f"{package['name']}-{version}.crate", "crate archive version/filename mismatch")
        return "crates.io", package["name"], "crate"
    if path.name.endswith(".tar.gz"):
        assets = {
            f"anbani-v{version}-x86_64-linux-musl.tar.gz": "cli-x86_64",
            f"anbani-v{version}-aarch64-linux-musl.tar.gz": "cli-aarch64",
            f"anbani-wasm-v{version}.tar.gz": "wasm",
            f"anbani-c-header-v{version}.tar.gz": "c-header",
        }
        if context.family == "rs" and path.name in assets:
            package = assets[path.name]
            with tarfile.open(path, "r:gz") as archive:
                regular = [m for m in archive.getmembers() if m.isfile()]
                if package.startswith("cli-"):
                    binaries = [m for m in regular if PurePosixPath(m.name).name == "anbani"]
                    require(len(binaries) == 1 and binaries[0].size > 64, "CLI asset lacks executable")
                    header = archive.extractfile(binaries[0]).read(64)
                    machine = 62 if package == "cli-x86_64" else 183
                    require(header[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", header, 18)[0] == machine, "CLI asset architecture mismatch")
                elif package == "c-header":
                    headers = [m for m in regular if PurePosixPath(m.name).name == "anbani.h"]
                    require(len(headers) == 1 and headers[0].size > 0, "header asset lacks anbani.h")
                    header = archive.extractfile(headers[0]).read(1024 * 1024)
                    require(b"anbani_call" in header and b"anbani_free" in header, "header asset lacks C ABI declarations")
                else:
                    require(any(m.name.endswith(".wasm") and m.size > 8 for m in regular), "wasm asset lacks wasm module")
                    require(any(m.name.endswith("/index.js") for m in regular), "wasm asset lacks wrapper")
                    package_json = [body for name, body in members.items() if name.endswith("/package.json")]
                    require(len(package_json) == 1, "wasm asset lacks unique package metadata")
                    metadata = json_read(package_json[0], path.name)
                    require(metadata.get("name") == "@anbani/wasm" and metadata.get("version") == version, "wasm download metadata mismatch")
            return "github", package, "asset"
        metadata = [body for name, body in members.items() if name.count("/") == 1 and name.endswith("/PKG-INFO")]
        require(len(metadata) == 1, "sdist lacks unique top-level PKG-INFO")
        info = BytesParser().parsebytes(metadata[0])
        package = normalize_name(info["Name"] or "")
        python = pypi_version(version)
        require(python_version(info["Version"]) == python, "sdist version mismatch")
        require(path.name in {f"{package}-{python}.tar.gz", f"{package.replace('-', '_')}-{python}.tar.gz"}, "sdist filename differs from PKG-INFO")
        return "pypi", package, "sdist"
    raise ReleaseError(f"unrecognized release payload: {path.name}")

def artifact_directory(path):
    path = Path(path)
    require(".." not in path.parts, "artifact destination must not contain parent traversal")
    if not path.is_absolute():
        path = Path.cwd() / path
    for ancestor in (*reversed(path.parents), path):
        require(not ancestor.is_symlink(), "artifact destination or ancestor must not be a symlink")
        require(not ancestor.exists() or ancestor.is_dir(), "artifact destination and ancestors must be directories")
    return path

def inspect_payloads(context, artifacts):
    artifacts = artifact_directory(artifacts)
    require(artifacts.is_dir() and not artifacts.is_symlink(), "artifacts must be a real flat directory")
    paths = sorted(artifacts.iterdir())
    require(all(p.is_file() and not p.is_symlink() for p in paths), "artifacts directory must contain only regular files")
    expected = {(i["registry"], i["package"]) for i in family_items(context)}
    seen = {}
    files = []
    for path in paths:
        if path.name in {MANIFEST, CHECKSUMS}:
            continue
        require(re.fullmatch(r"[A-Za-z0-9_.+\-]+", path.name) is not None, "unsafe payload filename")
        require(0 < path.stat().st_size <= MAX_DOWNLOAD, "payload is empty or exceeds size limit")
        registry, package, role = payload_identity(context, path)
        require((registry, package) in expected, f"unexpected family payload: {path.name}")
        roles = seen.setdefault((registry, package), set())
        require(role not in roles, f"duplicate family payload role: {package}/{role}")
        roles.add(role)
        files.append({"name": path.name, "sha256": digest(path), "registry": registry, "package": package})
    require(set(seen) == expected, "incomplete family payload membership: " + ", ".join(p for _, p in sorted(expected - set(seen))))
    for (registry, package), roles in seen.items():
        required = ({"sdist"} | WHEEL_PLATFORMS) if context.family == "rs" and registry == "pypi" else ({"sdist", "wheel"} if registry == "pypi" else {"crate"} if registry == "crates.io" else {"npm"} if registry == "npm" else {"asset"})
        require(roles == required, f"incomplete or unsupported payload roles: {package}; expected {sorted(required)}")
    return files

def create_manifest(context, artifacts):
    files = inspect_payloads(context, artifacts)
    manifest = {"schema": 1, "source_sha": context.source_sha, "family": context.family, "version": context.version, "files": files}
    (artifacts / MANIFEST).write_bytes(json_bytes(manifest))
    names = sorted([f["name"] for f in files] + [MANIFEST])
    (artifacts / CHECKSUMS).write_text("".join(f"{digest(artifacts / name)}  {name}\n" for name in names), encoding="utf-8")
    return manifest

def validate_manifest(context, artifacts):
    artifacts = artifact_directory(artifacts)
    manifest = json_read((artifacts / MANIFEST).read_bytes(), MANIFEST)
    require(set(manifest) == {"schema", "source_sha", "family", "version", "files"} and manifest["schema"] == 1, "invalid manifest schema")
    require((manifest["source_sha"], manifest["family"], manifest["version"]) == (context.source_sha, context.family, context.version), "manifest source/family/version mismatch")
    require(isinstance(manifest["files"], list) and all(isinstance(f, dict) and set(f) == {"name", "sha256", "registry", "package"} for f in manifest["files"]), "invalid manifest file records")
    files = inspect_payloads(context, artifacts)
    require(manifest["files"] == files, "manifest payload metadata/digest mismatch")
    expected = "".join(f"{digest(artifacts / name)}  {name}\n" for name in sorted([f["name"] for f in files] + [MANIFEST]))
    require((artifacts / CHECKSUMS).read_text(encoding="utf-8") == expected, "SHA256SUMS differs from payloads/manifest")
    return manifest

def verify_artifacts(context, artifacts, registry=None):
    artifacts = Path(artifacts)
    manifest = validate_manifest(context, artifacts)
    registry = registry or Registry()
    result = selection(context, registry)
    by_identity = {(i["registry"], i["package"]): i for i in result["items"]}
    missing = []
    for file in manifest["files"]:
        if file["registry"] != "github" and registry.file_state(by_identity[(file["registry"], file["package"])], file) == "missing":
            missing.append(file)
    result["missing_files"] = missing
    result["ready_to_finalize"] = not missing and not context.publication_holds
    return result

def download_draft(context, release, destination):
    assets = context.api.assets(release["id"])
    by_name = {}
    for asset in assets:
        name = asset.get("name", "")
        require(isinstance(name, str) and Path(name).name == name and re.fullmatch(r"[A-Za-z0-9_.+\-]+", name), "unsafe draft asset name")
        require(name not in by_name, "duplicate draft asset")
        by_name[name] = asset
    if MANIFEST not in by_name or CHECKSUMS not in by_name:
        return False
    raw = context.api.download(by_name[MANIFEST]["id"])
    manifest = json_read(raw, "stored manifest")
    require(manifest.get("schema") == 1 and manifest.get("source_sha") == context.source_sha and manifest.get("family") == context.family and manifest.get("version") == context.version, "stored manifest source/family/version mismatch")
    require(isinstance(manifest.get("files"), list), "stored manifest lacks files")
    expected = {MANIFEST, CHECKSUMS}
    for file in manifest["files"]:
        require(isinstance(file, dict) and isinstance(file.get("name"), str) and re.fullmatch(r"[A-Za-z0-9_.+\-]+", file["name"]), "unsafe stored payload name")
        require(file["name"] not in expected, "duplicate stored payload name")
        expected.add(file["name"])
    if not expected <= set(by_name):
        return False
    require(set(by_name) == expected, "draft contains assets outside stored manifest")
    for name in sorted(expected):
        body = raw if name == MANIFEST else context.api.download(by_name[name]["id"])
        (destination / name).write_bytes(body)
    validate_manifest(context, destination)
    return True

def install_authoritative(context, source, destination):
    destination = artifact_directory(destination)
    source_paths = list(source.iterdir())
    require(all(p.is_file() and not p.is_symlink() for p in source_paths), "authoritative artifacts must be flat regular files")
    destination.mkdir(parents=True, exist_ok=True)
    existing = list(destination.iterdir())
    require(all(p.is_file() and not p.is_symlink() for p in existing), "artifact destination must be flat regular files")
    source_names = {p.name for p in source_paths}
    rebuilt = [p for p in existing if p.name not in source_names]
    family = {(i["registry"], i["package"]) for i in family_items(context)}
    for path in rebuilt:
        try:
            require(re.fullmatch(r"[A-Za-z0-9_.+\-]+", path.name) is not None, "unsafe rebuilt filename")
            require(0 < path.stat().st_size <= MAX_DOWNLOAD, "rebuilt payload size exceeds limit")
            registry, package, _ = payload_identity(context, path)
            require((registry, package) in family, "rebuilt payload belongs to another family")
        except (ReleaseError, OSError, ValueError, KeyError, TypeError, UnicodeError) as error:
            raise ReleaseError("unrelated files outside authoritative staging namespace") from error
    for path in source_paths:
        shutil.copyfile(path, destination / path.name)
    for path in rebuilt:
        path.unlink()

def prepare(context, artifacts, registry=None):
    require_unheld(context)
    artifacts = artifact_directory(artifacts)
    require(git(context.root, "rev-parse", "HEAD") == context.source_sha, "prepare requires exact tagged checkout")
    registry = registry or Registry()
    state = selection(context, registry)
    release = context.api.release(context.tag)
    if release is not None:
        require(release.get("draft") is True, "release is already public; use verify/finalize for confirmation")
        with tempfile.TemporaryDirectory(prefix="anbani-stored-") as temporary:
            stored = Path(temporary).resolve()
            if download_draft(context, release, stored):
                result = verify_artifacts(context, stored, registry)
                install_authoritative(context, stored, artifacts)
                validate_manifest(context, artifacts)
                result["release_id"] = release["id"]
                return result
    require(all(i["status"] == "missing" for i in state["items"] if i["registry"] != "github"), "incomplete/no draft after registry publication; use a new patch version")
    create_manifest(context, artifacts)
    result = verify_artifacts(context, artifacts, registry)
    if release is None:
        context.api.existing_tag(context.root, context.tag)
        release = context.api.request("POST", f"/repos/{REPOSITORY}/releases", {"tag_name": context.tag, "target_commitish": context.source_sha, "name": context.tag, "draft": True, "prerelease": "-" in context.version.split("+", 1)[0]})
    else:
        for asset in context.api.assets(release["id"]):
            context.api.request("DELETE", f"/repos/{REPOSITORY}/releases/assets/{asset['id']}")
    for name in sorted(p.name for p in artifacts.iterdir()):
        context.api.request("POST", f"https://uploads.github.com/repos/{REPOSITORY}/releases/{release['id']}/assets?" + urlencode({"name": name}), data=(artifacts / name).read_bytes())
    with tempfile.TemporaryDirectory(prefix="anbani-upload-check-") as temporary:
        stored = Path(temporary).resolve()
        require(download_draft(context, release, stored), "uploaded draft is incomplete")
        require((stored / MANIFEST).read_bytes() == (artifacts / MANIFEST).read_bytes(), "uploaded manifest differs")
    result["release_id"] = release["id"]
    return result

def finalize(context, registry=None):
    require_unheld(context)
    require(git(context.root, "rev-parse", "HEAD") == context.source_sha, "finalize requires exact tagged checkout")
    release = context.api.release(context.tag)
    require(release is not None, "release draft does not exist")
    with tempfile.TemporaryDirectory(prefix="anbani-finalize-") as temporary:
        artifacts = Path(temporary).resolve()
        require(download_draft(context, release, artifacts), "release draft is incomplete")
        result = verify_artifacts(context, artifacts, registry)
        require(result["ready_to_finalize"], "registry publication is incomplete")
        run(context.root, [sys.executable, str(context.root / "packages/py/tools/release-build.py"), "registry-smoke", "--tag", context.tag], timeout=4500)
        result = verify_artifacts(context, artifacts, Registry() if registry is None else registry)
        require(result["ready_to_finalize"], "registry publication changed during consumer proof")
        require_unheld(load_context(context.root, context.tag, context.api))
        if release.get("draft"):
            context.api.request("PATCH", f"/repos/{REPOSITORY}/releases/{release['id']}", {"draft": False, "name": context.tag, "body": f"Source commit: {context.source_sha}\n\nAll registry digests and fresh registry consumers verified. See release-manifest.json and SHA256SUMS for the exact payload mapping."})
        for item in result["items"]:
            if item["registry"] == "github":
                item["status"] = "present"
        result.update({"release_id": release["id"], "finalized": True})
        return result

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("plan", "prepare", "verify", "finalize"):
        command = commands.add_parser(name)
        command.add_argument("--tag", required=True)
        if name in {"prepare", "verify"}:
            command.add_argument("--artifacts", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        context = load_context(ROOT, args.tag, check_remote=args.command != "verify")
        if args.command == "plan":
            result = plan(context)
        elif args.command == "verify":
            result = verify_artifacts(context, args.artifacts)
        elif args.command == "prepare":
            result = prepare(context, args.artifacts)
        else:
            result = finalize(context)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0
    except (ReleaseError, OSError, ValueError, KeyError, TypeError, UnicodeError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"release: {error}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    install_signal_handlers()
    raise SystemExit(main())
