import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
SPEC = importlib.util.spec_from_file_location("archive_smoke", Path(__file__).resolve().parents[1] / "archive-smoke.py")
archive = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(archive)

class SourceLockTests(unittest.TestCase):
    def test_reconciliation_allows_only_removing_packages(self):
        retained = {"name": "anbani-api", "version": "0.1.0"}
        original = {"package": [retained, {"name": "unused", "version": "1.0.0"}]}
        archive.validate_lock_pruning(original, {"package": [retained]})

    def test_reconciliation_rejects_version_checksum_source_or_new_package_changes(self):
        original = {"package": [{"name": "serde", "version": "1.0.229", "source": "registry+https://index.crates.io/", "checksum": "a" * 64}]}
        for changes in ({"version": "1.0.230"}, {"checksum": "b" * 64},
                        {"source": "registry+https://other.invalid/"}, {"name": "unexpected"}):
            changed = {"package": [dict(original["package"][0], **changes)]}
            with self.subTest(changes=changes), self.assertRaisesRegex(ValueError, "package pin"):
                archive.validate_lock_pruning(original, changed)

class RelocatedSourceTests(unittest.TestCase):

    def test_native_python_staging_joins_sources_without_mutating_owners(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "packages/rs"
            python = root / "packages/py"
            crate = rust / "crates/anbani-py"
            frontend = python / "anbani-rs"
            crate.mkdir(parents=True)
            frontend.mkdir(parents=True)
            (rust / "Cargo.toml").write_text('[workspace]\nmembers=["crates/anbani-py"]\n')
            (rust / "rust-toolchain.toml").write_text('[toolchain]\nchannel="1.96.0"\n')
            (crate / "Cargo.toml").write_text('[package]\nname="anbani-py"\nversion="0.1.0"\n')
            (crate / "build.rs").write_text("fn main() {}\n")
            (rust / "target").mkdir()
            (rust / "target/stale.whl").write_bytes(b"not a fresh build")
            metadata = ('[tool.maturin]\nmanifest-path = "../../rs/crates/anbani-py/Cargo.toml"\n'
                        'locked = true\nsdist-generator = "cargo"\n')
            (frontend / "pyproject.toml").write_text(metadata)
            for filename in ("LICENSE", "NOTICE", "README.md"):
                (frontend / filename).write_text("original " + filename)
            temporary = root / "outside"
            temporary.mkdir()
            with patch.object(archive, "RS_ROOT", rust), patch.object(archive, "PY_ROOT", python):
                workspace, project = archive.stage_rust_python(temporary, "stage")
            self.assertFalse((workspace / "target").exists())
            self.assertFalse((crate / "pyproject.toml").exists())
            self.assertEqual((frontend / "pyproject.toml").read_text(), metadata)

if __name__ == "__main__":
    unittest.main()
