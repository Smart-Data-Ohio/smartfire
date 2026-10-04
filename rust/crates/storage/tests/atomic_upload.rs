//! A failed disk upload must leave neither partial files nor damage to a completed blob.
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
fn failed_read_removes_the_staged_upload() {
    let root = tempfile::tempdir().unwrap();
    let service = DiskService::new(root.path(), "local");
    let key = "atomicuploadreadfailure";
    assert!(matches!(
        service.upload(key, FailingReader::new(), Some("ignored")),
        Err(Error::Io(_))
    ));
    assert!(!service.exist(key));
    assert!(
        filenames(&service, key).is_empty(),
        "partial staging file remains"
    );
}

#[test]
fn failed_retry_preserves_the_completed_blob() {
    let root = tempfile::tempdir().unwrap();
    let service = DiskService::new(root.path(), "local");
    let key = "atomicuploadretry";
    let original = b"previous complete blob";
    service
        .upload(key, original.as_slice(), Some(&key::checksum(original)))
        .unwrap();

    assert!(matches!(
        service.upload(key, FailingReader::new(), None),
        Err(Error::Io(_))
    ));
    assert_eq!(service.download(key).unwrap(), original);
    assert_eq!(filenames(&service, key), [key]);

    assert!(matches!(
        service.upload(
            key,
            b"corrupt retry".as_slice(),
            Some(&key::checksum(original))
        ),
        Err(Error::Integrity)
    ));
    assert_eq!(service.download(key).unwrap(), original);
    assert_eq!(filenames(&service, key), [key]);

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
fn checksum_mismatch_removes_only_the_staged_upload() {
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
