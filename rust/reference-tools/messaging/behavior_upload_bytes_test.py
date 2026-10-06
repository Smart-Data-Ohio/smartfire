from pathlib import Path
import tempfile
import unittest
from behavior_upload_bytes import uploaded_bytes


class UploadedBytesTest(unittest.TestCase):
    def test_reads_the_disk_service_file(self):
        contents = b"An attachment sent from the Markdown composer.\n"
        with tempfile.TemporaryDirectory() as directory:
            storage = Path(directory)
            path = storage / "36/fx/36fxkey"
            path.parent.mkdir(parents=True)
            path.write_bytes(contents)
            self.assertEqual(uploaded_bytes(storage, "36fxkey"), contents)

    def test_a_missing_file_is_failure_evidence_and_is_never_faked(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(FileNotFoundError):
                uploaded_bytes(Path(directory), "36fxkey")


if __name__ == "__main__":
    unittest.main()
