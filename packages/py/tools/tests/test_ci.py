
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
SPEC = importlib.util.spec_from_file_location("ci", Path(__file__).resolve().parents[1] / "ci.py")
ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ci)

class SelectionTests(unittest.TestCase):
    def test_each_pure_package_selects_only_its_language_and_archive(self):
        for package, path in (("js", "packages/js/anbani"), ("python", "packages/py/anbani"), ("textart", "packages/js/textart")):
            with self.subTest(package=package):
                selected = ci.select([f"{path}/src/changed-file"])
                for language in ("js", "python", "textart", "rust", "bindings", "native"):
                    self.assertEqual(selected[language], language == package)
                self.assertTrue(selected["generation"])
                self.assertTrue(selected["archives"])
                self.assertEqual(selected["archive_packages"], [package])
                self.assertEqual(selected["browser"], package != "python")
                self.assertFalse(selected["rust_archives"])

    def test_generated_package_output_still_requires_generation(self):
        for path in ("packages/py/anbani/anbani/data/letters.json", "packages/js/anbani/src/lib/data.mjs",
                     "packages/js/textart/src/fonts/BlockFont/BlockRegularDefinitions.mjs"):
            self.assertTrue(ci.select([path])["generation"])

    def test_rust_inputs_request_all_rust_distribution_consumers(self):
        for path in ("packages/rs/crates/anbani-core/src/lib.rs", "packages/js/native/index.cjs",
                     "packages/py/anbani-rs/pyproject.toml", "packages/js/wasm/index.js",
                     "packages/rs/Cargo.toml", "packages/rs/Cargo.lock", "packages/rs/rust-toolchain.toml", "packages/rs/.cargo/config.toml"):
            with self.subTest(path=path):
                selected = ci.select([path])
                for flag in ("rust", "bindings", "native", "rust_archives", "browser", "generation"):
                    self.assertTrue(selected[flag])
                self.assertFalse(selected["archives"])
                self.assertFalse(selected["js"])
                self.assertFalse(selected["python"])
                self.assertFalse(selected["textart"])

    def test_js_workspace_inputs_cover_both_js_packages_and_native_packing(self):
        for path in ("packages/js/package.json", "packages/js/package-lock.json"):
            selected = ci.select([path])
            for flag in ("js", "textart", "native", "bindings", "archives", "browser", "generation"):
                self.assertTrue(selected[flag])
            self.assertEqual(selected["archive_packages"], ["js", "textart"])
            self.assertFalse(selected["python"])
            self.assertFalse(selected["rust"])

    def test_common_unknown_and_missing_inputs_are_conservative(self):
        for paths in (None, ["common/data/scripts/letters.json"], ["common/spec/golden-art.json"],
                      ["packages/py/tools/generators/core.py"], ["packages/py/tools/archive-smoke.py"],
                      ["packages/js/tools/conformance.mjs"], [".github/workflows/release.yml"], [".github/Makefile"], ["new-build.toml"],
                      ["packages/new-engine/src/main.rs"], [".pre-commit-config.yaml"]):
            with self.subTest(paths=paths):
                selected = ci.select(paths)
                self.assertTrue(all(selected[flag] for flag in ci.FLAGS))
                self.assertEqual(selected["archive_packages"], ["js", "python", "textart"])

    def test_documentation_and_empty_changes_keep_only_always_running_generation(self):
        for paths in ([], ["README.md", "common/docs/architecture.md", "LICENSE"]):
            selected = ci.select(paths)
            self.assertEqual([flag for flag in ci.FLAGS if selected[flag]], ["generation"])
            self.assertEqual(selected["archive_packages"], [])

    def test_prefix_boundaries_do_not_ignore_unknown_inputs(self):
        self.assertTrue(ci.select(["packages/py/anbani-extra/build.json"])["rust"])
        self.assertTrue(ci.select(["docs-config.json"])["bindings"])

class GitChangesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="anbani-ci-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        self.write("README.md", "initial\n")
        self.base = self.commit()

    def git(self, *args):
        return subprocess.run(
            ["git", "-c", "user.name=CI selector test", "-c", "user.email=ci@example.invalid",
             *args], cwd=self.root, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            check=True, timeout=30,
        ).stdout.decode().strip()

    def write(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def commit(self):
        self.git("add", "--all")
        self.git("commit", "--no-gpg-sign", "-qm", "fixture")
        return self.git("rev-parse", "HEAD")

    def test_nul_paths_preserve_newlines_spaces_tabs_and_unicode(self):
        names = ["packages/js/anbani/src/line\nbreak.mjs", "packages/js/anbani/src/tab\tname.mjs",
                 "packages/py/anbani/tests/ქართული test.py", "packages/js/textart/test/trailing "]
        for name in names:
            self.write(name, "changed\n")
        head = self.commit()
        paths, _ = ci.changed_paths(self.root, self.base, head)
        self.assertEqual(set(paths), set(names))
        selected = ci.select(paths)
        self.assertEqual(selected["archive_packages"], ["js", "python", "textart"])
        self.assertFalse(selected["rust"])

    def test_move_and_delete_cover_old_and_new_consumers(self):
        self.write("packages/js/anbani/move.mjs", "same source\n")
        self.write("packages/rs/crates/anbani-core/delete.rs", "obsolete\n")
        base = self.commit()
        (self.root / "packages/py/anbani").mkdir(parents=True)
        (self.root / "packages/js/anbani/move.mjs").rename(self.root / "packages/py/anbani/move.py")
        (self.root / "packages/rs/crates/anbani-core/delete.rs").unlink()
        head = self.commit()
        paths, _ = ci.changed_paths(self.root, base, head)
        self.assertEqual(set(paths), {"packages/js/anbani/move.mjs", "packages/py/anbani/move.py",
                                      "packages/rs/crates/anbani-core/delete.rs"})
        selected = ci.select(paths)
        self.assertTrue(selected["js"])
        self.assertTrue(selected["python"])
        self.assertTrue(selected["rust"])

    def test_missing_zero_unknown_and_invalid_bases_select_all(self):
        for base in ("", "0" * 40, "f" * 40, "--output=untrusted", "HEAD; touch bad"):
            paths, _ = ci.changed_paths(self.root, base, self.base)
            self.assertIsNone(paths)
            self.assertTrue(all(ci.select(paths)[flag] for flag in ci.FLAGS))

    def test_invalid_or_unavailable_head_is_an_error(self):
        for head in ("", "0" * 40, "f" * 40, "HEAD", "--help", "$(touch bad)"):
            with self.subTest(head=head), self.assertRaises(ValueError):
                ci.changed_paths(self.root, self.base, head)

    def test_equal_commits_have_no_scoped_jobs(self):
        paths, _ = ci.changed_paths(self.root, self.base, self.base)
        self.assertEqual(paths, [])
        self.assertFalse(ci.select(paths)["rust"])

class AggregateGateTests(unittest.TestCase):
    def results(self, selection):
        requested = {
            "changes": True, "generation": True, "js": selection["js"],
            "python": selection["python"], "textart": selection["textart"],
            "rust": selection["rust"], "bindings": selection["bindings"],
            "pure-archives": selection["archives"], "rust-archives": selection["rust_archives"],
            "browser": selection["browser"],
        }
        return {job: {"result": "success" if needed else "skipped"}
                for job, needed in requested.items()}

    def test_skipped_unaffected_jobs_do_not_block(self):
        for selection in (ci.select([]), ci.select(["packages/py/anbani/anbani/cli.py"]), ci.select(None)):
            self.assertEqual(ci.gate_failures(selection, self.results(selection)), [])

    def test_any_requested_failure_cancellation_skip_or_missing_result_blocks(self):
        selection = ci.select(None)
        for job in self.results(selection):
            for state in ("failure", "cancelled", "skipped", "missing"):
                with self.subTest(job=job, state=state):
                    results = self.results(selection)
                    if state == "missing":
                        del results[job]
                    else:
                        results[job]["result"] = state
                    self.assertTrue(ci.gate_failures(selection, results))

    def test_selector_or_generation_failure_blocks_documentation_only_gate(self):
        selection = ci.select(["README.md"])
        for job in ("changes", "generation"):
            results = self.results(selection)
            results[job]["result"] = "failure"
            self.assertTrue(ci.gate_failures(selection, results))

    def test_without_bindings_drops_only_bindings_and_records_it(self):
        full = ci.select(None)
        selection = ci.without(full, "bindings")
        self.assertFalse(selection["bindings"])
        self.assertFalse(selection["native"])
        self.assertEqual(selection["without"], ["bindings"])
        self.assertEqual({k: v for k, v in selection.items() if k not in ("bindings", "native", "without")},
                         {k: v for k, v in full.items() if k not in ("bindings", "native")})
        self.assertEqual(ci.gate_failures(selection, self.results(selection)), [])
        self.assertNotIn("without", ci.without(full, ""))

    def test_without_rejects_jobs_a_runner_could_host(self):
        for names in ("rust", "bindings,js", "browser"):
            with self.subTest(names=names), self.assertRaises(ValueError):
                ci.without(ci.select(None), names)

    def test_native_alone_still_requires_binding_result(self):
        selection = ci.select([])
        selection["native"] = True
        self.assertTrue(ci.gate_failures(selection, self.results(selection)))

if __name__ == "__main__":
    unittest.main()
