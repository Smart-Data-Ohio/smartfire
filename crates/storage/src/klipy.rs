//! Server-owned provenance for imported KLIPY media blobs.

use crate::{Blob, Json, Verifier};

pub const MARK_KEY: &str = "klipy_mark";
const MARK_PURPOSE: &str = "klipy_import";

pub fn import_mark(verifier: &dyn Verifier, blob_key: &str) -> String {
    verifier.generate(&Json::String(blob_key.into()).encode(), MARK_PURPOSE, None)
}

pub fn verified_import(verifier: &dyn Verifier, blob: &Blob, now: jiff::Timestamp) -> bool {
    matches!(blob.content_type.as_deref(), Some("image/gif" | "image/webp" | "video/mp4"))
        && blob.metadata.get(MARK_KEY).and_then(Json::as_str).is_some_and(|mark| {
            verifier.verified(mark, MARK_PURPOSE, now)
                .is_some_and(|data| data == Json::String(blob.key.clone()).encode())
        })
}

pub fn verified_image_import(verifier: &dyn Verifier, blob: &Blob, now: jiff::Timestamp) -> bool {
    matches!(blob.content_type.as_deref(), Some("image/gif" | "image/webp"))
        && verified_import(verifier, blob, now)
}
