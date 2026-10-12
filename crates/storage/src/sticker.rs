//! Sticker limits over the shared emoji decoder and first-frame variants.

use crate::{
    Filename, Storage,
    branding::{self, Kind, PreparedEmoji},
    vips::Cancellation,
};

pub const MAX_BYTES: i64 = 512 * 1024;

pub fn prepare(
    storage: &Storage,
    key: &str,
    filename: &Filename,
    byte_size: i64,
    cancel: &Cancellation,
) -> Result<PreparedEmoji, String> {
    cancel.check().map_err(|_| "could not be read")?;
    let path = storage.service.path_for(key);
    let size = std::fs::metadata(&path)
        .map_err(|_| "could not be read")?
        .len();
    if byte_size > MAX_BYTES || size > MAX_BYTES as u64 {
        return Err("must be 512 KB or smaller".into());
    }
    if byte_size <= 0 || size != byte_size as u64 {
        return Err("could not be read".into());
    }
    let bytes = std::fs::read(&path).map_err(|_| "could not be read")?;
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        let canvas = bytes.get(6..10).ok_or("could not be read")?;
        if u16::from_le_bytes([canvas[0], canvas[1]]) > 320
            || u16::from_le_bytes([canvas[2], canvas[3]]) > 320
        {
            return Err(Kind::Sticker.dimension_message().into());
        }
        if crate::workspace_icon::gif_frames(&bytes).ok_or("could not be read")? > 100 {
            return Err("must contain at most 100 frames".into());
        }
    }
    let mut prepared = branding::prepare_expression(
        storage,
        key,
        filename,
        Kind::Sticker,
        MAX_BYTES as u64,
        cancel,
    )?;
    if prepared.content_type == "image/png" {
        prepared.still = branding::png_still(storage, filename, &bytes, cancel)?;
        prepared.metadata.set(
            crate::workspace_icon::ANIMATED_KEY,
            crate::Json::Bool(prepared.still.is_some()),
        );
    }
    Ok(prepared)
}
