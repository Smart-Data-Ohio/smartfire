from pathlib import Path
import tempfile
import unittest
from behavior_upload_bytes import uploaded_bytes


class UploadedBytesTest(unittest.TestCase):
    def test_reads_local_file_and_the_real_pinned_test_service(self):
        contents = b"An attachment sent from the Markdown composer.\n"
        with tempfile.TemporaryDirectory() as directory:
            storage = Path(directory)
            path = storage / "36/fx/36fxkey"
            path.parent.mkdir(parents=True)
            path.write_bytes(contents)
            self.assertEqual(uploaded_bytes(storage, "36fxkey"), contents)
            seen = []

            def container(arguments):
                seen.append(arguments)
                return contents

            self.assertEqual(uploaded_bytes(storage, "36fxkey", rails_test_port=22020,
                                            read_container=container), contents)
            self.assertEqual(seen, [["docker", "exec", "ws8bm-behavior-reference-22020",
                                     "cat", "/rails/tmp/storage/36/fx/36fxkey"]])

    def test_a_missing_file_is_failure_evidence_and_is_never_faked(self):
        def missing(_arguments):
            raise FileNotFoundError("missing uploaded bytes")

        with self.assertRaisesRegex(FileNotFoundError, "missing uploaded bytes"):
            uploaded_bytes(Path("unused"), "36fxkey", rails_test_port=22020,
                           read_container=missing)


if __name__ == "__main__":
    unittest.main()
