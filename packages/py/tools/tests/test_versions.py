import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import versions  # noqa: E402
from repo_paths import REPO_ROOT  # noqa: E402

class VersionTests(unittest.TestCase):
    def test_manifests_match_source(self):
        self.assertEqual(versions.run(REPO_ROOT, write=False), [])

    def test_shared_contract(self):
        families = versions.load(REPO_ROOT)["families"]
        self.assertEqual(len({tuple(families[f].split(".")[:2]) for f in versions.SHARED}), 1)

if __name__ == "__main__":
    unittest.main()
