import base64
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
SPEC = importlib.util.spec_from_file_location("anbani_release", Path(__file__).resolve().parents[1] / "release.py")
release = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release
SPEC.loader.exec_module(release)

def archive(path, files):
    with tarfile.open(path, "w:gz") as out:
        for name, content in files.items():
            content = content.encode() if isinstance(content, str) else content
            info = tarfile.TarInfo(name)
            info.size = len(content)
            info.mtime = 0
            out.addfile(info, io.BytesIO(content))
    return path

def npm_payload(directory, *, name="anbani", version="3.4.1", text="export default 'ქართული';\n"):
    filename = name.replace("@", "").replace("/", "-") + "-" + version + ".tgz"
    return archive(directory / filename, {
        "package/package.json": json.dumps({"name": name, "version": version, "main": "index.js"}),
        "package/index.js": text, "package/LICENSE": "GPL", "package/NOTICE": "Attribution",
    })

def python_payloads(directory, version="3.4.1", *, package="anbani", platform=None):
    metadata = f"Metadata-Version: 2.1\nName: {package}\nVersion: {version}\n\nPython implementation\n"
    distribution = package.replace("-", "_")
    sdist = archive(directory / f"{distribution}-{version}.tar.gz", {
        f"{distribution}-{version}/PKG-INFO": metadata,
        f"{distribution}-{version}/{distribution}/__init__.py": "value = 'ქართული'\n",
    })
    tag = "cp39-abi3-" + platform if platform else "py3-none-any"
    wheel = directory / f"{distribution}-{version}-{tag}.whl"
    with zipfile.ZipFile(wheel, "w") as out:
        out.writestr(f"{distribution}-{version}.dist-info/METADATA", metadata)
        out.writestr(f"{distribution}-{version}.dist-info/WHEEL", f"Wheel-Version: 1.0\nTag: {tag}\n")
        out.writestr("anbani/__init__.py", "value = 'ქართული'\n")
    return sdist, wheel

class FixtureHTTP:
    def __init__(self):
        self.responses = {}
        self.calls = []

    def add(self, url, value=None, status=200, method="GET"):
        body = value if isinstance(value, bytes) else json.dumps(value).encode()
        self.responses[(method, url)] = (status, {}, body)

    def request(self, method, url, **kwargs):
        self.calls.append((method, url, kwargs))
        if (method, url) not in self.responses:
            raise AssertionError(f"unexpected fixture HTTP request: {method} {url}")
        response = self.responses[(method, url)]
        if isinstance(response, Exception):
            raise response
        return response

class DraftHTTP(FixtureHTTP):
    def __init__(self, tag, tag_object):
        super().__init__()
        self.tag = tag
        self.tag_object = tag_object
        self.record = None
        self.assets = {}
        self.next_id = 20

    def seed(self, directory, *, omit=()):
        self.record = {"id": 7, "tag_name": self.tag, "draft": True}
        for path in directory.iterdir():
            if path.name not in omit:
                self.store(path.name, path.read_bytes())

    def store(self, name, body):
        asset_id = self.next_id
        self.next_id += 1
        self.assets[asset_id] = ({"id": asset_id, "name": name}, body)

    def request(self, method, url, **kwargs):
        self.calls.append((method, url, kwargs))
        parts = urlsplit(url)
        prefix = "/repos/Anbani/anbani/releases"
        path = parts.path
        if method == "GET" and path == "/repos/Anbani/anbani/git/ref/tags/" + self.tag:
            return 200, {}, json.dumps({"ref": "refs/tags/" + self.tag, "object": {"sha": self.tag_object}}).encode()
        if method == "GET" and path == prefix:
            return 200, {}, json.dumps([self.record] if self.record else []).encode()
        if method == "GET" and path == prefix + "/7/assets":
            return 200, {}, json.dumps([row for row, _ in self.assets.values()]).encode()
        if path.startswith(prefix + "/assets/"):
            asset_id = int(path.rsplit("/", 1)[1])
            if method == "GET":
                return 200, {}, self.assets[asset_id][1]
            if method == "DELETE":
                del self.assets[asset_id]
                return 204, {}, b""
        if method == "POST" and path == prefix:
            payload = json.loads(kwargs["data"])
            self.record = {"id": 7, **payload}
            return 201, {}, json.dumps(self.record).encode()
        if method == "POST" and parts.netloc == "uploads.github.com" and path == prefix + "/7/assets":
            self.store(parse_qs(parts.query)["name"][0], kwargs["data"])
            return 201, {}, json.dumps(self.assets[self.next_id - 1][0]).encode()
        if method == "PATCH" and path == prefix + "/7":
            self.record.update(json.loads(kwargs["data"]))
            return 200, {}, json.dumps(self.record).encode()
        raise AssertionError(f"unexpected GitHub fixture route: {method} {url}")

class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="anbani-release-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve() / "source"
        self.root.mkdir()
        self.artifacts = Path(self.temporary.name).resolve() / "artifacts"
        self.artifacts.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Release Fixture")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "tag.gpgsign", "false")
        self.write("packages/js/anbani/package.json", {"name": "anbani", "version": "3.4.1"})
        self.write("packages/js/textart/package.json", {"name": "anbani-textart", "version": "1.8.1"})
        self.write("packages/py/anbani/pyproject.toml", '[project]\nname="anbani"\nversion="3.4.1"\n')
        self.write("common/data/manifest.json", {"publication_holds": []})
        self.commit()
        self.sha = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", self.sha)
        self.git("tag", "-a", "js-v3.4.1", "-m", "fixture annotated release")
        self.context = release.Context(self.root, "js-v3.4.1", "js", "3.4.1", self.sha, [])

    def git(self, *arguments):
        result = subprocess.run(["git", *arguments], cwd=self.root, check=True, capture_output=True, text=True, timeout=15)
        return result.stdout.strip()

    def write(self, name, value):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value) if isinstance(value, dict) else value, encoding="utf-8")

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-m", "fixture input")

    def gate(self, *, sha=None, main=None, success=True, tag="js-v3.4.1"):
        sha = sha or self.sha
        http = FixtureHTTP()
        prefix = "https://api.github.com/repos/Anbani/anbani"
        http.add(prefix + "/git/ref/tags/" + tag, {"ref": "refs/tags/" + tag, "object": {"sha": self.git("rev-parse", "refs/tags/" + tag)}})
        http.add(prefix + "/commits/main", {"sha": main or self.sha})
        http.add(prefix + f"/actions/workflows/ci.yml/runs?head_sha={sha}&per_page=100&page=1", {"workflow_runs": [{"head_sha": sha, "event": "push", "status": "completed", "conclusion": "success", "check_suite_id": 55}]})
        http.add(prefix + f"/commits/{sha}/check-runs?per_page=100&page=1", {"check_runs": [{"name": "ci-gate", "head_sha": sha, "status": "completed", "conclusion": "success" if success else "failure", "app": {"slug": "github-actions"}, "check_suite": {"id": 55}}]})
        return release.GitHub(http, "fixture-token"), http

    def npm_registry(self, payload=None, *, version_status=404, identity_status=404):
        http = FixtureHTTP()
        base = "https://registry.npmjs.org/anbani"
        http.add(base, {"name": "anbani"}, identity_status)
        if payload is not None:
            body = payload.read_bytes()
            tarball = base + "/-/anbani-3.4.1.tgz"
            metadata = {"name": "anbani", "version": "3.4.1", "dist": {"tarball": tarball, "integrity": "sha512-" + base64.b64encode(hashlib.sha512(body).digest()).decode()}}
            http.add(base + "/3.4.1", metadata)
            http.add(tarball, body)
        else:
            http.add(base + "/3.4.1", {}, version_status)
        return release.Registry(http), http

    def draft(self):
        http = DraftHTTP(self.context.tag, self.git("rev-parse", "refs/tags/" + self.context.tag))
        self.context.api = release.GitHub(http, "fixture-token")
        return http

    def rust_tag(self, *, dependency_version="0.10.1", descriptor_version="0.10.1", lock_version="0.10.1"):
        names = list(release.CRATES) + ["anbani-py", "anbani-napi", "anbani-cabi", "anbani-wasm"]
        members = ["crates/" + name for name in names]
        cargo = '[workspace]\nmembers=' + json.dumps(members) + '\n[workspace.package]\nversion="0.10.1"\n[workspace.dependencies]\n'
        for name in release.CRATES:
            cargo += f'{name}={{path="crates/{name}",version="{dependency_version}"}}\n'
        self.write("packages/rs/Cargo.toml", cargo)
        for name in names:
            manifest = f'[package]\nname="{name}"\nversion.workspace=true\n'
            if name not in release.CRATES:
                manifest += "publish=false\n"
            if name != "anbani-core":
                manifest += "[dependencies]\nanbani-core.workspace=true\n"
            self.write(f"packages/rs/crates/{name}/Cargo.toml", manifest)
        self.write("packages/rs/Cargo.lock", "version=4\n" + "".join(f'[[package]]\nname="{name}"\nversion="{lock_version}"\n' for name in names))
        self.write("packages/rs/rust-toolchain.toml", '[toolchain]\nchannel="1.96.0"\n')
        self.write("packages/js/native/package.json", {"name": "@anbani/native", "version": descriptor_version, "private": True})
        self.commit()
        self.sha = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", self.sha)
        self.git("tag", "-a", "rs-v0.10.1", "-m", "fixture Rust release")

    def test_rust_tag_checks_workspace_pins_descriptor_and_lock(self):
        self.rust_tag()
        api, _ = self.gate(tag="rs-v0.10.1")
        context = release.load_context(self.root, "rs-v0.10.1", api)
        self.assertEqual((context.family, context.version, context.source_sha), ("rs", "0.10.1", self.sha))

    def test_rust_dependency_pin_mismatch_before_http(self):
        self.rust_tag(dependency_version="0.10.0")
        api, http = self.gate(tag="rs-v0.10.1")
        with self.assertRaisesRegex(release.ReleaseError, "dependency pin"):
            release.load_context(self.root, "rs-v0.10.1", api)
        self.assertEqual(http.calls, [])

    def test_rust_descriptor_mismatch_before_http(self):
        self.rust_tag(descriptor_version="0.10.0")
        api, http = self.gate(tag="rs-v0.10.1")
        with self.assertRaisesRegex(release.ReleaseError, "descriptor"):
            release.load_context(self.root, "rs-v0.10.1", api)
        self.assertEqual(http.calls, [])

    def test_rust_lock_mismatch_before_http(self):
        self.rust_tag(lock_version="0.10.0")
        api, http = self.gate(tag="rs-v0.10.1")
        with self.assertRaisesRegex(release.ReleaseError, "Cargo.lock"):
            release.load_context(self.root, "rs-v0.10.1", api)
        self.assertEqual(http.calls, [])

    def test_remote_tag_object_mismatch_refused(self):
        api, http = self.gate()
        http.add("https://api.github.com/repos/Anbani/anbani/git/ref/tags/js-v3.4.1", {"ref": "refs/tags/js-v3.4.1", "object": {"sha": self.sha}})
        with self.assertRaisesRegex(release.ReleaseError, "tag object"):
            release.load_context(self.root, "js-v3.4.1", api)

    def test_successful_gate_for_other_sha_or_workflow_not_accepted(self):
        api, http = self.gate()
        endpoint = f"https://api.github.com/repos/Anbani/anbani/commits/{self.sha}/check-runs?per_page=100&page=1"
        http.add(endpoint, {"check_runs": [{"name": "ci-gate", "head_sha": "f" * 40, "status": "completed", "conclusion": "success", "app": {"slug": "github-actions"}, "check_suite": {"id": 55}}]})
        with self.assertRaisesRegex(release.ReleaseError, "ci-gate"):
            release.load_context(self.root, "js-v3.4.1", api)
        http.add(endpoint, {"check_runs": [{"name": "ci-gate", "head_sha": self.sha, "status": "completed", "conclusion": "success", "app": {"slug": "github-actions"}, "check_suite": {"id": 999}}]})
        with self.assertRaisesRegex(release.ReleaseError, "ci-gate"):
            release.load_context(self.root, "js-v3.4.1", api)

    def test_tag_semver_strictness(self):
        for invalid in ("v3.4.1", "js-v03.4.1", "js-v3.4", "js-v3.4.1-01", "js-v3.4.1/evil", "rs-v1.2.3\n"):
            with self.subTest(invalid=invalid), self.assertRaises(release.ReleaseError):
                release.parse_tag(invalid)
        self.assertEqual(release.parse_tag("rs-v1.2.3-rc.1+build.5"), ("rs", "1.2.3-rc.1+build.5"))

    def test_annotated_tag_dereferences_and_exact_ci(self):
        api, http = self.gate()
        result = release.load_context(self.root, "js-v3.4.1", api)
        self.assertEqual(result.source_sha, self.sha)
        self.assertNotEqual(self.git("rev-parse", "refs/tags/js-v3.4.1"), self.sha)
        self.assertTrue(all(call[0] == "GET" for call in http.calls))

    def test_version_mismatch_before_http(self):
        self.git("tag", "js-v3.4.2")
        api, http = self.gate()
        with self.assertRaisesRegex(release.ReleaseError, "manifest"):
            release.load_context(self.root, "js-v3.4.2", api)
        self.assertEqual(http.calls, [])

    def test_non_main_tag_and_failed_ci_refused(self):
        self.git("checkout", "-b", "off-main")
        self.write("other", "divergent")
        self.commit()
        branch_sha = self.git("rev-parse", "HEAD")
        self.git("tag", "js-v3.4.1-off.1")
        api, _ = self.gate(sha=branch_sha, main=self.sha)
        with self.assertRaises(release.ReleaseError):
            api.safety(self.root, branch_sha)
        self.git("checkout", "main")
        api, _ = self.gate(success=False)
        with self.assertRaisesRegex(release.ReleaseError, "ci-gate"):
            release.load_context(self.root, "js-v3.4.1", api)

    def test_verify_context_has_no_github_access(self):
        result = release.load_context(self.root, "js-v3.4.1", check_remote=False)
        self.assertEqual(result.source_sha, self.sha)
        self.assertIsNone(result.api)

    def test_families_are_independent_and_rs_complete(self):
        expected = {"js": {("npm", "anbani")}, "py": {("pypi", "anbani")}, "textart": {("npm", "anbani-textart")}}
        for family, identities in expected.items():
            self.context.family = family
            self.assertEqual({(i["registry"], i["package"]) for i in release.family_items(self.context)}, identities)
        self.context.family = "rs"
        items = release.family_items(self.context)
        self.assertEqual(len([i for i in items if i["registry"] == "npm"]), 7)
        self.assertEqual([i["package"] for i in items if i["registry"] == "crates.io"], list(release.CRATES))
        self.assertEqual(len([i for i in items if i["registry"] == "github"]), 4)

    def test_plan_missing_and_bootstrap_read_only(self):
        registry, http = self.npm_registry()
        result = release.plan(self.context, registry)
        self.assertEqual(result["items"][0]["status"], "missing")
        self.assertEqual(result["bootstrap_required"], [{"registry": "npm", "package": "anbani"}])
        self.assertTrue(all(method == "GET" for method, _, _ in http.calls))
        self.assertEqual(list(self.artifacts.iterdir()), [])

    def test_rs_plan_probes_all_registries_and_draft_asset_ids(self):
        self.context.family = "rs"
        self.context.version = "0.10.1"
        self.context.tag = "rs-v0.10.1"
        self.git("tag", self.context.tag)
        draft = self.draft()
        draft.record = {"id": 7, "tag_name": self.context.tag, "draft": True}
        draft.store("anbani-v0.10.1-x86_64-linux-musl.tar.gz", b"asset-list fixture")
        http = FixtureHTTP()
        for item in release.family_items(self.context):
            name, version = item["package"], item["version"]
            if item["registry"] == "npm":
                base = "https://registry.npmjs.org/" + release.quote(name, safe="")
                http.add(base, {}, 404)
                http.add(base + "/" + version, {}, 404)
            elif item["registry"] == "crates.io":
                base = "https://crates.io/api/v1/crates/" + name
                http.add(base, {}, 404)
                http.add(base + "/" + version, {}, 404)
            elif item["registry"] == "pypi":
                http.add(f"https://pypi.org/pypi/{name}/{version}/json", {}, 404)
                http.add(f"https://pypi.org/pypi/{name}/json", {}, 404)
        result = release.plan(self.context, release.Registry(http))
        self.assertEqual(len(result["bootstrap_required"]), 13)
        assets = {i["package"]: i["status"] for i in result["items"] if i["registry"] == "github"}
        self.assertEqual(assets["cli-x86_64"], "present")
        self.assertEqual(assets["cli-aarch64"], "missing")
        self.assertTrue(all(method == "GET" for method, _, _ in draft.calls + http.calls))

    def test_existing_identity_missing_version_not_bootstrap(self):
        registry, _ = self.npm_registry(identity_status=200)
        result = release.selection(self.context, registry)
        self.assertEqual(result["bootstrap_required"], [])
        self.assertEqual(result["items"][0]["status"], "missing")

    def test_new_version_must_outrank_every_published_version(self):
        for published, allowed in ((["3.4.0", "3.4.1-rc.1"], True), (["3.4.1"], False), (["3.5.0"], False), (["3.4.10"], False), (["4.0.0-rc.1"], False)):
            with self.subTest(published=published):
                http = FixtureHTTP()
                http.add("https://registry.npmjs.org/anbani", {"name": "anbani", "versions": {v: {} for v in published}})
                http.add("https://registry.npmjs.org/anbani/3.4.1", {}, 404)
                if allowed:
                    self.assertEqual(release.selection(self.context, release.Registry(http))["items"][0]["status"], "missing")
                else:
                    with self.assertRaisesRegex(release.ReleaseError, "does not outrank"):
                        release.selection(self.context, release.Registry(http))
        self.assertGreater(release.version_key("3.4.1"), release.version_key("3.4.1rc1"))
        self.assertGreater(release.version_key("3.4.10"), release.version_key("3.4.9"))
        registry = release.Registry(FixtureHTTP())
        registry.http.add("https://pypi.org/pypi/anbani/json", {"releases": {"3.4.0": [], "3.5.0": []}})
        with self.assertRaisesRegex(release.ReleaseError, "pypi anbani 3.4.1 does not outrank published 3.5.0"):
            registry.require_newer({"registry": "pypi", "package": "anbani", "version": "3.4.1"})
        registry.listings[("crates.io", "anbani-core")] = release.published_versions("crates.io", {"versions": [{"num": "0.10.1", "yanked": True}]})
        registry.require_newer({"registry": "crates.io", "package": "anbani-core", "version": "4.0.0"})

    def test_status_precedence_never_treats_auth_as_absence(self):
        for status in (401, 403, 429, 500):
            with self.subTest(status=status):
                registry, _ = self.npm_registry(version_status=status, identity_status=404)
                with self.assertRaisesRegex(release.ReleaseError, f"HTTP {status}"):
                    release.selection(self.context, registry)
                registry, _ = self.npm_registry(version_status=404, identity_status=status)
                with self.assertRaisesRegex(release.ReleaseError, f"HTTP {status}"):
                    release.selection(self.context, registry)

    def test_network_error_not_missing(self):
        registry, http = self.npm_registry()
        http.responses[("GET", "https://registry.npmjs.org/anbani")] = release.ReleaseError("HTTP connection failed or timed out")
        with self.assertRaises(release.ReleaseError):
            release.selection(self.context, registry)

    def test_manifest_actual_bytes_and_missing_selection(self):
        payload = npm_payload(self.artifacts)
        manifest = release.create_manifest(self.context, self.artifacts)
        registry, _ = self.npm_registry()
        result = release.verify_artifacts(self.context, self.artifacts, registry)
        self.assertEqual(result["missing_files"], [{"name": payload.name, "sha256": hashlib.sha256(payload.read_bytes()).hexdigest(), "registry": "npm", "package": "anbani"}])
        self.assertEqual(manifest["source_sha"], self.sha)
        sums = (self.artifacts / release.CHECKSUMS).read_text()
        self.assertIn(release.MANIFEST, sums)
        self.assertNotIn(release.CHECKSUMS, sums)
        self.assertFalse(result["ready_to_finalize"])

    def test_registry_present_matching_digest_and_conflict(self):
        payload = npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        registry, _ = self.npm_registry(payload, identity_status=200)
        result = release.verify_artifacts(self.context, self.artifacts, registry)
        self.assertEqual(result["missing_files"], [])
        self.assertTrue(result["ready_to_finalize"])
        other = Path(self.temporary.name) / "other"
        other.mkdir()
        changed = npm_payload(other, text="different published bytes")
        registry, _ = self.npm_registry(changed, identity_status=200)
        with self.assertRaisesRegex(release.ReleaseError, "digest conflict"):
            release.verify_artifacts(self.context, self.artifacts, registry)

    def test_payload_tamper_manifest_source_and_metadata_refused(self):
        payload = npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        payload.write_bytes(payload.read_bytes() + b"tampered")
        with self.assertRaisesRegex(release.ReleaseError, "digest"):
            release.validate_manifest(self.context, self.artifacts)
        npm_payload(self.artifacts, version="3.4.2")
        with self.assertRaisesRegex(release.ReleaseError, "version"):
            release.create_manifest(self.context, self.artifacts)

    def test_manifest_source_and_checksums_are_mandatory(self):
        npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        manifest_path = self.artifacts / release.MANIFEST
        document = json.loads(manifest_path.read_bytes())
        document["source_sha"] = "f" * 40
        manifest_path.write_bytes(release.json_bytes(document))
        with self.assertRaisesRegex(release.ReleaseError, "source/family/version"):
            release.validate_manifest(self.context, self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        (self.artifacts / release.CHECKSUMS).write_text("wrong checksum\n")
        with self.assertRaisesRegex(release.ReleaseError, "SHA256SUMS"):
            release.validate_manifest(self.context, self.artifacts)

    def test_plan_reports_hold_without_mutation(self):
        self.context.publication_holds = [{"id": "rights", "status": "open"}]
        registry, http = self.npm_registry()
        result = release.selection(self.context, registry)
        self.assertEqual(result["publication_holds"], self.context.publication_holds)
        self.assertTrue(all(method == "GET" for method, _, _ in http.calls))
        self.assertEqual(list(self.artifacts.iterdir()), [])

    def test_missing_native_platform_fails_before_registry(self):
        self.context.family = "rs"
        self.context.version = "0.10.1"
        npm_payload(self.artifacts, name="@anbani/wasm", version="0.10.1")
        with self.assertRaisesRegex(release.ReleaseError, "incomplete family") as error:
            release.create_manifest(self.context, self.artifacts)
        self.assertIn("@anbani/native-linux-arm64-gnu", str(error.exception))
        self.assertFalse((self.artifacts / release.MANIFEST).exists())

    def test_pypi_partial_version_selects_only_missing_file(self):
        self.context.family = "py"
        self.context.tag = "py-v3.4.1"
        sdist, wheel = python_payloads(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        http = FixtureHTTP()
        http.add("https://pypi.org/pypi/anbani/3.4.1/json", {"info": {"name": "anbani", "version": "3.4.1"}, "urls": [{"filename": sdist.name, "digests": {"sha256": release.digest(sdist)}}]})
        result = release.verify_artifacts(self.context, self.artifacts, release.Registry(http))
        self.assertEqual(result["items"][0]["status"], "present")
        self.assertEqual([f["name"] for f in result["missing_files"]], [wheel.name])
        http.add("https://pypi.org/pypi/anbani/3.4.1/json", {"info": {"name": "anbani", "version": "3.4.1"}, "urls": [{"filename": sdist.name, "digests": {"sha256": "0" * 64}}]})
        with self.assertRaisesRegex(release.ReleaseError, "digest conflict"):
            release.verify_artifacts(self.context, self.artifacts, release.Registry(http))

    def test_python_versions_normalize_without_changing_prerelease_meaning(self):
        mappings = {
            "0.10.2": "0.10.2", "0.10.2-rc.1": "0.10.2rc1",
            "0.10.2-alpha.2": "0.10.2a2", "0.10.2-beta.3": "0.10.2b3",
            "0.10.2-a1": "0.10.2a1", "0.10.2-b1": "0.10.2b1",
            "0.10.2-dev.4": "0.10.2.dev4", "0.10.2-rc.1.dev.2": "0.10.2rc1.dev2",
            "0.10.2rc01": "0.10.2rc1", "0.10.2-ALPHA.2": "0.10.2a2",
            "0.10.2+Build-ABC.001": "0.10.2+build.abc.1",
            "0.10.2-rc.1+linux.002": "0.10.2rc1+linux.2",
        }
        for source, expected in mappings.items():
            with self.subTest(source=source):
                self.assertEqual(release.python_version(source), expected)
        for invalid in ("0.10.2-nightly.1", "0.10.2-post.1", "0.10.2-1", "0.10.2-rc.1.extra", "0.10.2+bad!"):
            with self.subTest(source=invalid), self.assertRaises(release.ReleaseError):
                release.python_version(invalid)

    def test_pypi_items_normalize_but_top_level_and_other_registries_keep_semver(self):
        self.context.family, self.context.version = "rs", "0.10.2-rc.1"
        items = release.family_items(self.context)
        self.assertEqual({i["version"] for i in items if i["registry"] == "pypi"}, {"0.10.2rc1"})
        self.assertEqual({i["version"] for i in items if i["registry"] != "pypi"}, {"0.10.2-rc.1"})
        self.assertEqual(self.context.version, "0.10.2-rc.1")
        for invalid in ("0.10.2+linux.1", "0.10.2-nightly.1", "0.10.2-post.1"):
            self.context.version = invalid
            with self.subTest(version=invalid), self.assertRaises(release.ReleaseError):
                release.family_items(self.context)

    def test_unpublishable_python_versions_refused_before_git_or_http(self):
        api, http = self.gate()
        for family in ("py", "rs"):
            for version in ("0.10.2+linux.1", "0.10.2-nightly.1", "0.10.2-post.1"):
                with self.subTest(family=family, version=version), self.assertRaises(release.ReleaseError):
                    release.load_context(self.root, f"{family}-v{version}", api)
        self.assertEqual(http.calls, [])

    def test_npm_build_metadata_cannot_plan_a_different_registry_identity(self):
        for family, package in (("js", "anbani"), ("textart", "anbani-textart")):
            version = "3.4.2+build.1"
            self.write(f"packages/{family}/package.json", {"name": package, "version": version})
            self.commit()
            self.git("update-ref", "refs/remotes/origin/main", self.git("rev-parse", "HEAD"))
            tag = f"{family}-v{version}"
            self.git("tag", tag)
            with self.subTest(family=family), self.assertRaises(release.ReleaseError):
                release.load_context(self.root, tag, check_remote=False)

    def test_python_manifest_matches_normalized_prerelease_before_http(self):
        self.write("packages/py/anbani/pyproject.toml", '[project]\nname="anbani"\nversion="3.4.2rc1"\n')
        self.commit()
        self.sha = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", self.sha)
        self.git("tag", "py-v3.4.2-rc.1")
        context = release.load_context(self.root, "py-v3.4.2-rc.1", check_remote=False)
        self.assertEqual(context.version, "3.4.2-rc.1")
        self.assertEqual(release.family_items(context)[0]["version"], "3.4.2rc1")
        self.git("tag", "py-v3.4.2-rc.2")
        with self.assertRaisesRegex(release.ReleaseError, "Python manifest"):
            release.load_context(self.root, "py-v3.4.2-rc.2", check_remote=False)

    def test_prerelease_archives_and_pypi_per_file_digest_selection(self):
        for semver, normalized in (("3.4.2-rc.1", "3.4.2rc1"), ("3.4.2-alpha.2", "3.4.2a2"), ("3.4.2-beta.3", "3.4.2b3"), ("3.4.2-dev.4", "3.4.2.dev4")):
            with self.subTest(version=semver), tempfile.TemporaryDirectory(dir=self.artifacts.parent) as temporary:
                artifacts = Path(temporary)
                self.context.family, self.context.version, self.context.tag = "py", semver, "py-v" + semver
                sdist, wheel = python_payloads(artifacts, normalized)
                manifest = release.create_manifest(self.context, artifacts)
                self.assertEqual(manifest["version"], semver)
                http = FixtureHTTP()
                endpoint = f"https://pypi.org/pypi/anbani/{normalized}/json"
                http.add(endpoint, {"info": {"name": "anbani", "version": normalized}, "urls": [{"filename": sdist.name, "digests": {"sha256": release.digest(sdist)}}]})
                result = release.verify_artifacts(self.context, artifacts, release.Registry(http))
                self.assertEqual(result["version"], semver)
                self.assertEqual(result["items"][0]["version"], normalized)
                self.assertEqual([f["name"] for f in result["missing_files"]], [wheel.name])
                http.add(endpoint, {"info": {"name": "anbani", "version": normalized}, "urls": [{"filename": sdist.name, "digests": {"sha256": "0" * 64}}]})
                with self.assertRaisesRegex(release.ReleaseError, "digest conflict"):
                    release.verify_artifacts(self.context, artifacts, release.Registry(http))
                http.add(endpoint, {"info": {"name": "anbani", "version": "3.4.2rc9"}, "urls": []})
                with self.assertRaisesRegex(release.ReleaseError, "wrong package/version"):
                    release.verify_artifacts(self.context, artifacts, release.Registry(http))

    def test_maturin_prerelease_wheel_and_sdist_metadata(self):
        self.context.family, self.context.version = "rs", "0.10.2-rc.1"
        sdist, wheel = python_payloads(self.artifacts, "0.10.2rc1", package="anbani-rs", platform="manylinux_2_28_aarch64")
        self.assertEqual(release.payload_identity(self.context, sdist), ("pypi", "anbani-rs", "sdist"))
        self.assertEqual(release.payload_identity(self.context, wheel), ("pypi", "anbani-rs", "manylinux_2_28_aarch64"))
        self.context.version = "0.10.2-rc.2"
        for path in (sdist, wheel):
            with self.subTest(path=path.name), self.assertRaisesRegex(release.ReleaseError, "version mismatch"):
                release.payload_identity(self.context, path)

    def test_python_archive_filename_must_match_normalized_metadata(self):
        self.context.family, self.context.version = "py", "3.4.2-rc.1"
        sdist, wheel = python_payloads(self.artifacts, "3.4.2rc1")
        for path in (sdist, wheel):
            renamed = path.with_name(path.name.replace("3.4.2rc1", "3.4.2rc2"))
            path.rename(renamed)
            with self.subTest(path=renamed.name), self.assertRaisesRegex(release.ReleaseError, "filename"):
                release.payload_identity(self.context, renamed)

    def test_hold_refuses_prepare_finalize_without_http_or_writes(self):
        http = self.draft()
        self.context.publication_holds = [{"id": "approval", "status": "open"}]
        for command in (lambda: release.prepare(self.context, self.artifacts), lambda: release.finalize(self.context)):
            with self.assertRaisesRegex(release.ReleaseError, "held"):
                command()
        self.assertEqual(http.calls, [])
        self.assertEqual(list(self.artifacts.iterdir()), [])

    def test_local_hold_applies_to_older_tag(self):
        self.write("common/data/manifest.json", {"publication_holds": [{"id": "new-approval", "status": "open"}]})
        result = release.load_context(self.root, "js-v3.4.1", check_remote=False)
        self.assertEqual(result.publication_holds[0]["id"], "new-approval")

    def test_prepare_creates_draft_for_existing_tag_and_uploads_id_bytes(self):
        payload = npm_payload(self.artifacts)
        expected = payload.read_bytes()
        http = self.draft()
        registry, _ = self.npm_registry()
        result = release.prepare(self.context, self.artifacts, registry)
        self.assertEqual(result["release_id"], 7)
        self.assertTrue(http.record["draft"])
        self.assertEqual({row["name"] for row, _ in http.assets.values()}, {payload.name, release.MANIFEST, release.CHECKSUMS})
        stored = next(body for row, body in http.assets.values() if row["name"] == payload.name)
        self.assertEqual(stored, expected)
        self.assertTrue(any("/releases/assets/" in url for method, url, _ in http.calls if method == "GET"))
        self.assertFalse(any("/git/refs" in url for _, url, _ in http.calls))

    def test_complete_draft_resume_preserves_original_published_bytes(self):
        payload = npm_payload(self.artifacts)
        original = payload.read_bytes()
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts)
        published_dir = Path(self.temporary.name) / "published"
        published_dir.mkdir()
        published = published_dir / payload.name
        published.write_bytes(original)
        npm_payload(self.artifacts, text="rebuilt replacement must be discarded")
        registry, _ = self.npm_registry(published, identity_status=200)
        result = release.prepare(self.context, self.artifacts, registry)
        self.assertEqual(payload.read_bytes(), original)
        self.assertEqual(result["missing_files"], [])
        self.assertFalse(any(method != "GET" for method, _, _ in http.calls))

    def test_complete_draft_discards_valid_rebuild_with_different_filename(self):
        self.git("tag", "py-v3.4.1")
        self.context.family, self.context.tag = "py", "py-v3.4.1"
        sdist, wheel = python_payloads(self.artifacts)
        original = wheel.read_bytes()
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts)
        rebuilt = wheel.with_name(wheel.name.replace("-py3-", "-1-py3-"))
        wheel.rename(rebuilt)
        self.assertEqual(release.payload_identity(self.context, rebuilt), ("pypi", "anbani", "wheel"))
        registry_http = FixtureHTTP()
        registry_http.add("https://pypi.org/pypi/anbani/3.4.1/json", {"info": {"name": "anbani", "version": "3.4.1"}, "urls": [{"filename": sdist.name, "digests": {"sha256": release.digest(sdist)}}]})
        result = release.prepare(self.context, self.artifacts, release.Registry(registry_http))
        self.assertEqual(wheel.read_bytes(), original)
        self.assertFalse(rebuilt.exists())
        self.assertEqual([f["name"] for f in result["missing_files"]], [wheel.name])
        self.assertFalse(any(method != "GET" for method, _, _ in http.calls))

    def test_cli_complete_draft_refuses_symlink_and_symlink_ancestor_without_deletion(self):
        payload = npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts)
        target = self.artifacts.parent / "unrelated"
        target.mkdir()
        sentinel = target / "user-data.txt"
        sentinel.write_bytes(b"keep this unrelated file")
        ancestor_target = self.artifacts.parent / "ancestor-target"
        nested = ancestor_target / "nested"
        nested.mkdir(parents=True)
        nested_sentinel = nested / "other.txt"
        nested_sentinel.write_bytes(b"also keep this")
        link = self.artifacts.parent / "caller-link"
        link.symlink_to(target, target_is_directory=True)
        ancestor_link = self.artifacts.parent / "ancestor-link"
        ancestor_link.symlink_to(ancestor_target, target_is_directory=True)
        before = {row["name"]: body for row, body in http.assets.values()}
        for destination in (link, ancestor_link / "nested", ancestor_link / "missing"):
            registry, _ = self.npm_registry(payload, identity_status=200)
            with self.subTest(destination=destination), patch.object(release, "load_context", return_value=self.context), patch.object(release, "Registry", return_value=registry), patch("sys.stderr", new_callable=io.StringIO):
                self.assertEqual(release.main(["prepare", "--tag", self.context.tag, "--artifacts", str(destination)]), 1)
            self.assertTrue(link.is_symlink())
            self.assertTrue(ancestor_link.is_symlink())
            self.assertEqual(sentinel.read_bytes(), b"keep this unrelated file")
            self.assertEqual(nested_sentinel.read_bytes(), b"also keep this")
            self.assertFalse((ancestor_target / "missing").exists())
        self.assertEqual({row["name"]: body for row, body in http.assets.values()}, before)
        self.assertFalse(any(method != "GET" for method, _, _ in http.calls))

    def test_complete_draft_refuses_unrelated_staging_files_without_deletion(self):
        payload = npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts)
        sentinel = self.artifacts / "unrelated.txt"
        sentinel.write_bytes(b"must not be deleted")
        npm_payload(self.artifacts, text="rebuilt but preserve on refusal")
        before = {p.name: p.read_bytes() for p in self.artifacts.iterdir()}
        registry, _ = self.npm_registry()
        with self.assertRaisesRegex(release.ReleaseError, "outside.*staging|unrelated"):
            release.prepare(self.context, self.artifacts, registry)
        self.assertEqual({p.name: p.read_bytes() for p in self.artifacts.iterdir()}, before)
        self.assertFalse(any(method != "GET" for method, _, _ in http.calls))

    def test_incomplete_after_publication_refused_without_remote_mutation(self):
        payload = npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts, omit=(payload.name,))
        registry, _ = self.npm_registry(payload, identity_status=200)
        with self.assertRaisesRegex(release.ReleaseError, "incomplete/no draft"):
            release.prepare(self.context, self.artifacts, registry)
        self.assertTrue(all(method == "GET" for method, _, _ in http.calls))

    def test_incomplete_unpublished_draft_replaced_and_read_back(self):
        payload = npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts, omit=(payload.name,))
        registry, _ = self.npm_registry()
        result = release.prepare(self.context, self.artifacts, registry)
        self.assertEqual(result["release_id"], 7)
        self.assertTrue(any(method == "DELETE" for method, _, _ in http.calls))
        self.assertEqual({row["name"] for row, _ in http.assets.values()}, {payload.name, release.MANIFEST, release.CHECKSUMS})

    def test_stored_conflict_refused_without_overwriting_rebuild(self):
        payload = npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts)
        rebuilt = npm_payload(self.artifacts, text="keep rebuild when conflict found")
        before = rebuilt.read_bytes()
        registry, _ = self.npm_registry(rebuilt, identity_status=200)
        with self.assertRaisesRegex(release.ReleaseError, "digest conflict"):
            release.prepare(self.context, self.artifacts, registry)
        self.assertEqual(rebuilt.read_bytes(), before)
        self.assertTrue(all(method == "GET" for method, _, _ in http.calls))

    def test_finalize_requires_complete_registry_before_consumer(self):
        npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts)
        registry, _ = self.npm_registry()
        with self.assertRaisesRegex(release.ReleaseError, "incomplete"):
            release.finalize(self.context, registry)
        self.assertTrue(http.record["draft"])
        self.assertTrue(all(method == "GET" for method, _, _ in http.calls))

    def test_failed_actual_consumer_process_keeps_draft_private(self):
        payload = npm_payload(self.artifacts)
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts)
        self.write("packages/py/tools/release-build.py", "raise SystemExit(9)\n")
        registry, _ = self.npm_registry(payload, identity_status=200)
        with self.assertRaisesRegex(release.ReleaseError, "exit 9"):
            release.finalize(self.context, registry)
        self.assertTrue(http.record["draft"])
        self.assertFalse(any(method == "PATCH" for method, _, _ in http.calls))

    def test_run_failure_does_not_disclose_captured_subprocess_body(self):
        secret = "fixture-secret-body-that-must-not-escape"
        with self.assertRaisesRegex(release.ReleaseError, "exit 9") as failure:
            release.run(self.root, [sys.executable, "-c", f"import sys; print({secret!r}); print({secret!r}, file=sys.stderr); sys.exit(9)"])
        self.assertNotIn(secret, str(failure.exception))

    @unittest.skipUnless(os.name == "posix", "POSIX independently owned session regression")
    def test_run_timeout_unwinds_nested_owned_consumer_before_cleanup(self):
        pid_file = self.root / "nested.pid"
        cleanup_file = self.root / "cleanup"
        leaf = f"import os,time; from pathlib import Path; Path({str(pid_file)!r}).write_text(str(os.getpid())); time.sleep(60)"
        tools = str(Path(__file__).resolve().parents[1])
        self.write("consumer.py",
                   "import sys\nfrom pathlib import Path\n"
                   f"sys.path.insert(0, {tools!r})\n"
                   "from processes import run, install_signal_handlers\n"
                   "install_signal_handlers()\n"
                   "try:\n"
                   f"    run([sys.executable, '-c', {leaf!r}], timeout=60, capture_output=True)\n"
                   "finally:\n"
                   f"    Path({str(cleanup_file)!r}).write_text('unwound')\n")
        try:
            with self.assertRaisesRegex(release.ReleaseError, "failed or timed out"):
                release.run(self.root, [sys.executable, str(self.root / "consumer.py")], timeout=3)
            self.assertTrue(pid_file.exists(), "nested consumer must actually start")
            pid = int(pid_file.read_text())
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)
            self.assertEqual(cleanup_file.read_text(), "unwound")
        finally:
            if pid_file.exists():
                try:
                    os.kill(int(pid_file.read_text()), signal.SIGKILL)
                except ProcessLookupError:
                    pass

    def test_paginated_authenticated_release_lookup(self):
        http = FixtureHTTP()
        base = "https://api.github.com/repos/Anbani/anbani/releases"
        http.add(base + "?per_page=100&page=1", [{"id": n, "tag_name": f"old-{n}"} for n in range(100)])
        http.add(base + "?per_page=100&page=2", [{"id": 150, "tag_name": "js-v3.4.1", "draft": True}])
        api = release.GitHub(http, "fixture-secret")
        self.assertEqual(api.release("js-v3.4.1")["id"], 150)
        self.assertEqual(len(http.calls), 2)
        self.assertTrue(all(call[2]["headers"]["Authorization"] == "Bearer fixture-secret" for call in http.calls))

    def test_partial_pypi_complete_draft_resume_keeps_stored_bytes(self):
        self.git("tag", "py-v3.4.1")
        self.context.family = "py"
        self.context.tag = "py-v3.4.1"
        sdist, wheel = python_payloads(self.artifacts)
        original = {p.name: p.read_bytes() for p in (sdist, wheel)}
        release.create_manifest(self.context, self.artifacts)
        http = self.draft()
        http.seed(self.artifacts)
        wheel.write_bytes(b"untrusted rebuilt replacement")
        registry_http = FixtureHTTP()
        registry_http.add("https://pypi.org/pypi/anbani/3.4.1/json", {"info": {"name": "anbani", "version": "3.4.1"}, "urls": [{"filename": sdist.name, "digests": {"sha256": hashlib.sha256(original[sdist.name]).hexdigest()}}]})
        result = release.prepare(self.context, self.artifacts, release.Registry(registry_http))
        self.assertEqual([f["name"] for f in result["missing_files"]], [wheel.name])
        self.assertEqual(wheel.read_bytes(), original[wheel.name])
        self.assertEqual(sdist.read_bytes(), original[sdist.name])
        self.assertTrue(all(method == "GET" for method, _, _ in http.calls))

    def test_crates_checksum_and_auth_status(self):
        item = {"registry": "crates.io", "package": "anbani-core", "version": "0.10.1"}
        http = FixtureHTTP()
        base = "https://crates.io/api/v1/crates/anbani-core"
        http.add(base, {"crate": {"id": "anbani-core"}})
        http.add(base + "/0.10.1", {"version": {"crate": "anbani-core", "num": "0.10.1", "checksum": "a" * 64}})
        file = {"name": "anbani-core-0.10.1.crate", "sha256": "a" * 64}
        self.assertEqual(release.Registry(http).file_state(item, file), "present")
        file["sha256"] = "b" * 64
        with self.assertRaisesRegex(release.ReleaseError, "digest conflict"):
            release.Registry(http).file_state(item, file)
        http.add(base + "/0.10.1", {}, 403)
        with self.assertRaisesRegex(release.ReleaseError, "HTTP 403"):
            release.Registry(http).probe(item)

if __name__ == "__main__":
    unittest.main()
