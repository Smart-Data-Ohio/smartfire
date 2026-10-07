//! Bounded workspace image validation. Only GIF and WebP advertise animation; APNG is static.

use std::io::Read;
use std::path::Path;

use crate::{Blob, Filename, Json, Staged, Storage, Variation, vips::Image};

pub const MAX_BYTES: u64 = 10 * 1024 * 1024;
/// A frame's canvas counts even when its encoded update covers only a small rectangle.
/// At most 100 megapixels across all frames, checked before evaluating any pixels.
pub const MAX_ANIMATION_PIXELS: u64 = 100_000_000;
pub const ANIMATED_KEY: &str = "branding_animated";

#[derive(Clone, Copy)]
pub enum Kind {
    Logo,
    Banner,
}

impl Kind {
    pub fn maximum_dimensions(self) -> (i32, i32) {
        match self {
            Self::Logo => (4096, 4096),
            Self::Banner => (4096, 2304),
        }
    }

    pub fn dimension_message(self) -> &'static str {
        match self {
            Self::Logo => "must be at most 4096 × 4096 pixels",
            Self::Banner => "must be at most 4096 × 2304 pixels",
        }
    }

    pub fn still_variation(self) -> Variation {
        let (width, height) = match self {
            Self::Logo => (512, 512),
            Self::Banner => (1920, 1080),
        };
        Variation::resize_to_limit(width, height, Some("png"))
    }
}

pub enum Invalid {
    Format,
    Size,
    Dimensions,
    AnimationBudget,
}

impl Invalid {
    pub fn message(&self, kind: Kind) -> &'static str {
        match self {
            Self::Format => "must be a PNG, JPEG, GIF or WebP image",
            Self::Size => "must be 10 MB or smaller",
            Self::Dimensions => kind.dimension_message(),
            Self::AnimationBudget => "must contain at most 100 megapixels across all frames",
        }
    }
}

pub struct Prepared {
    pub blob: Blob,
    pub still: Option<(Variation, Staged)>,
}

/// Missing metadata (older or classic uploads) always means static. No file is opened.
pub fn animated(blob: &Blob) -> bool {
    matches!(blob.metadata.get(ANIMATED_KEY), Some(Json::Bool(true)))
}

/// Must run on the media blocking pool. The strict, single-page loader only reads headers
/// until dimensions and the total animation budget have been checked.
pub fn prepare(storage: &Storage, mut blob: Blob, kind: Kind) -> Result<Prepared, Invalid> {
    let path = storage.service.path_for(&blob.key);
    let (image, content_type) = header(&path, kind)?;
    blob.content_type = Some(content_type.to_owned());
    blob.metadata.set("identified", Json::Bool(true));
    let pages = if matches!(content_type, "image/gif" | "image/webp") {
        image
            .get_string("n-pages")
            .and_then(|n| n.parse::<u64>().ok())
    } else {
        Some(1)
    };
    let still = if pages.is_some_and(|n| n > 1) {
        let variation = kind.still_variation();
        // A damaged frame degrades to static. Rendering never tries this extraction again.
        transform(storage, &blob, image, &variation)
            .ok()
            .map(|staged| (variation, staged))
    } else {
        None
    };
    blob.metadata.set(ANIMATED_KEY, Json::Bool(still.is_some()));
    Ok(Prepared { blob, still })
}

/// The same bounds also apply to a static variant requested for a classic/older attachment.
pub fn transform_variant(
    storage: &Storage,
    blob: &Blob,
    kind: Kind,
    variation: &Variation,
) -> crate::Result<Staged> {
    let (image, _) = header(&storage.service.path_for(&blob.key), kind)
        .map_err(|invalid| crate::Error::Analyze(invalid.message(kind).to_owned()))?;
    transform(storage, blob, image, variation)
}

fn header(path: &Path, kind: Kind) -> Result<(Image, &'static str), Invalid> {
    let mut file = std::fs::File::open(path).map_err(|_| Invalid::Format)?;
    if file.metadata().map_err(|_| Invalid::Format)?.len() > MAX_BYTES {
        return Err(Invalid::Size);
    }
    let mut magic = [0; 12];
    file.read_exact(&mut magic).map_err(|_| Invalid::Format)?;
    let content_type = if magic.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if magic.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if magic.starts_with(b"GIF87a") || magic.starts_with(b"GIF89a") {
        "image/gif"
    } else if magic.starts_with(b"RIFF") && &magic[8..] == b"WEBP" {
        "image/webp"
    } else {
        return Err(Invalid::Format);
    };
    let image = Image::open_branding(path, content_type).map_err(|_| Invalid::Format)?;
    let (width, height) = (image.width(), image.height());
    if width <= 0 || height <= 0 {
        return Err(Invalid::Format);
    }
    let (max_width, max_height) = kind.maximum_dimensions();
    if width > max_width || height > max_height {
        return Err(Invalid::Dimensions);
    }
    if matches!(content_type, "image/gif" | "image/webp") {
        let pages = image
            .get_string("n-pages")
            .and_then(|n| n.parse::<u64>().ok());
        if pages.is_some_and(|pages| {
            pages
                .checked_mul(width as u64 * height as u64)
                .is_none_or(|pixels| pixels > MAX_ANIMATION_PIXELS)
        }) {
            return Err(Invalid::AnimationBudget);
        }
    }
    Ok((image, content_type))
}

fn transform(
    storage: &Storage,
    blob: &Blob,
    image: Image,
    variation: &Variation,
) -> crate::Result<Staged> {
    let format = variation.format()?;
    let image = image.autorot()?;
    let image = match variation
        .transformations()
        .iter()
        .find(|(key, _)| key == "resize_to_limit")
    {
        Some((_, crate::marshal::Value::Array(dimensions))) => {
            let dimension = |i| match dimensions.get(i) {
                Some(crate::marshal::Value::Int(n)) => Some(*n as i32),
                _ => None,
            };
            image.resize_to_limit(dimension(0), dimension(1))?
        }
        _ => {
            return Err(crate::Error::InvalidVariation(
                "expected resize_to_limit".into(),
            ));
        }
    };
    let output = tempfile::Builder::new()
        .prefix("branding-")
        .suffix(&format!(".{format}"))
        .tempfile()?;
    image.write_to_file(output.path())?;
    storage.stage_analyzed(
        output.path(),
        Filename::new(format!("{}.{format}", blob.filename.base())),
        &variation.content_type()?,
    )
}
