import importlib.util
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
SPEC = importlib.util.spec_from_file_location("anbani_publish", Path(__file__).resolve().parents[1] / "release-publish.py")
publisher = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(publisher)

class PublicationWindowTests(unittest.TestCase):
    def test_failed_job_resume_uses_attempt_start_not_expired_prepare(self):
        attempt = {"head_sha": "a" * 40, "run_started_at": "2026-11-05T19:00:00Z"}
        remaining = publisher.publication_remaining(1.0, attempt, "a" * 40, 1793905260.0)
        self.assertEqual(remaining, 1140.0)

    def test_later_job_consumes_the_same_attempt_budget(self):
        attempt = {"head_sha": "a" * 40, "run_started_at": "2026-11-05T19:00:00Z"}
        self.assertEqual(publisher.publication_remaining(1, attempt, "a" * 40, 1793905800), 600)
        with self.assertRaisesRegex(RuntimeError, "deadline exceeded"):
            publisher.publication_remaining(1, attempt, "a" * 40, 1793906400)

    def test_fresh_prepare_after_long_rebuild_wins(self):
        attempt = {"head_sha": "a" * 40, "run_started_at": "2026-11-05T19:00:00Z"}
        self.assertEqual(publisher.publication_remaining(1793908800, attempt, "a" * 40, 1793907600), 1200)

    def test_normal_attempt_never_resets_the_prepare_deadline(self):
        self.assertEqual(publisher.publication_remaining(1793906400, None, "a" * 40, 1793905800), 600)
        with self.assertRaisesRegex(RuntimeError, "deadline exceeded"):
            publisher.publication_remaining(1793906400, None, "a" * 40, 1793906401)

    def test_rerun_cannot_borrow_another_source_or_an_untrusted_clock(self):
        for attempt in (
            {"head_sha": "b" * 40, "run_started_at": "2026-11-05T19:00:00Z"},
            {"head_sha": "a" * 40, "run_started_at": "2026-11-05T19:00:00"},
            {"head_sha": "a" * 40, "run_started_at": "2026-11-05T20:00:00Z"},
        ):
            with self.subTest(attempt=attempt), self.assertRaises(ValueError):
                publisher.publication_remaining(1, attempt, "a" * 40, 1793905260)

    def test_nonfinite_deadline_cannot_disable_the_bound(self):
        for deadline in (float("inf"), float("nan")):
            with self.subTest(deadline=deadline), self.assertRaises(ValueError):
                publisher.publication_remaining(deadline, None, "a" * 40, 1793905260)

if __name__ == "__main__":
    unittest.main()
