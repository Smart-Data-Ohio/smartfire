//! Disk upload failures follow pinned Rails, without leaking staging files.
use std::io::{self, Read};

use campfire_storage::{DiskService, Error, key};

struct FailingReader(io::Cursor<&'static [u8]>);

impl FailingReader {
    fn new() -> Self {
        Self(io::Cursor::new(b"partial body"))
    }
}

impl Read for FailingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.0.read(buffer)? {
            0 => Err(io::Error::other("injected source read failure")),
            count => Ok(count),
        }
    }
}

fn filenames(service: &DiskService, key: &str) -> Vec<String> {
    std::fs::read_dir(service.path_for(key).parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

#[test]
fn source_read_failures_match_rails_copied_bytes_for_fresh_uploads_and_retries() {
    let vector: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/direct_upload_limits.json")).unwrap();
    let cases: Vec<_> = vector["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["name"].as_str().unwrap().starts_with("reader_error"))
        .collect();
    assert_eq!(cases.len(), 2);
    for case in cases {
        let root = tempfile::tempdir().unwrap();
        let service = DiskService::new(root.path(), "local");
        let key = case["name"].as_str().unwrap();
        if let Some(previous) = case["previous_body"].as_str() {
            service
                .upload(
                    key,
                    previous.as_bytes(),
                    Some(&key::checksum(previous.as_bytes())),
                )
                .unwrap();
        }
        assert!(matches!(
            service.upload(key, FailingReader::new(), Some("ignored")),
            Err(Error::Io(_))
        ));
        assert_eq!(case["status"], 500);
        assert_eq!(service.exist(key), case["file_exists"]);
        assert_eq!(
            service.download(key).unwrap(),
            case["stored_body"].as_str().unwrap().as_bytes()
        );
        assert_eq!(filenames(&service, key), [key]);
    }
}

#[test]
fn corrupt_retry_deletes_the_completed_blob_and_can_be_retried() {
    let root = tempfile::tempdir().unwrap();
    let service = DiskService::new(root.path(), "local");
    let key = "atomicuploadretry";
    let original = b"previous complete blob";
    service
        .upload(key, original.as_slice(), Some(&key::checksum(original)))
        .unwrap();

    assert!(matches!(
        service.upload(
            key,
            b"corrupt retry".as_slice(),
            Some(&key::checksum(original))
        ),
        Err(Error::Integrity)
    ));
    assert!(!service.exist(key));
    assert!(filenames(&service, key).is_empty());

    let replacement = b"new complete blob";
    service
        .upload(
            key,
            replacement.as_slice(),
            Some(&key::checksum(replacement)),
        )
        .unwrap();
    assert_eq!(service.download(key).unwrap(), replacement);
    assert_eq!(filenames(&service, key), [key]);
}

#[test]
fn unwritable_destination_does_not_publish_a_blob() {
    let root = tempfile::tempdir().unwrap();
    let service = DiskService::new(root.path(), "local");
    let key = "atomicuploadunwritable";
    let path = service.path_for(key);
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent.parent().unwrap()).unwrap();
    // A file blocks the destination directory, regardless of the test user's permissions.
    std::fs::write(parent, b"not a writable directory").unwrap();
    assert!(matches!(
        service.upload(key, b"body".as_slice(), None),
        Err(Error::Io(_))
    ));
    assert!(!path.exists());
    assert_eq!(std::fs::read(parent).unwrap(), b"not a writable directory");
}

#[test]
fn failed_publication_removes_the_verified_staging_file() {
    let root = tempfile::tempdir().unwrap();
    let service = DiskService::new(root.path(), "local");
    let key = "atomicuploadrenamefailure";
    std::fs::create_dir_all(service.path_for(key)).unwrap();
    let body = b"body";
    assert!(matches!(
        service.upload(key, body.as_slice(), Some(&key::checksum(body))),
        Err(Error::Io(_))
    ));
    assert!(service.path_for(key).is_dir());
    assert_eq!(
        filenames(&service, key),
        [key],
        "verified staging file remains"
    );
}

#[test]
fn checksum_mismatch_removes_the_blob_and_staged_upload() {
    let root = tempfile::tempdir().unwrap();
    let service = DiskService::new(root.path(), "local");
    let key = "atomicuploadchecksumfailure";
    assert!(matches!(
        service.upload(key, b"body".as_slice(), Some(&key::checksum(b"other"))),
        Err(Error::Integrity)
    ));
    assert!(!service.exist(key));
    assert!(filenames(&service, key).is_empty());
}

#[cfg(unix)]
#[test]
fn write_failures_match_rails_partial_files_without_staging_leaks() {
    const CHILD: &str = "CAMPFIRE_UPLOAD_WRITE_FAILURE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "write_failures_match_rails_partial_files_without_staging_leaks",
            ])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let vector: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/direct_upload_limits.json")).unwrap();
    // Only this test subprocess changes its signal handler and resource limit.
    unsafe {
        libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
        assert_eq!(
            libc::setrlimit(
                libc::RLIMIT_FSIZE,
                &libc::rlimit {
                    rlim_cur: 4096,
                    rlim_max: 4096
                }
            ),
            0
        );
    }
    let cases: Vec<_> = vector["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["name"].as_str().unwrap().starts_with("write_error"))
        .collect();
    assert_eq!(cases.len(), 2);
    for case in cases {
        let root = tempfile::tempdir().unwrap();
        let service = DiskService::new(root.path(), "local");
        let key = case["name"].as_str().unwrap();
        if let Some(previous) = case["previous_body"].as_str() {
            service.upload(key, previous.as_bytes(), None).unwrap();
        }
        let result = service.upload(key, &vec![b'x'; 65536][..], Some("ignored"));
        assert!(
            matches!(&result, Err(Error::Io(error)) if error.kind() == io::ErrorKind::FileTooLarge),
            "{result:?}"
        );
        assert_eq!(case["status"], 500);
        assert_eq!(service.exist(key), case["file_exists"]);
        let path = service.path_for(key);
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            case["stored_bytes"]
        );
        assert_eq!(key::checksum_file(&path).unwrap(), case["stored_checksum"]);
        assert_eq!(filenames(&service, key), [key]);
    }
}
