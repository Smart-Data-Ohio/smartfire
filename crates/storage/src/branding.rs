//! Bounded workspace image validation. Only GIF and WebP advertise animation; APNG is static.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use crate::{
    Blob, Filename, Json, Staged, Storage, Variation,
    vips::{Cancellation, Image},
};

/// Bound encoded input for both uploads and legacy conversions: small canvases can still
/// carry large metadata that a loader buffers before the pixel budget can be checked.
pub const MAX_BYTES: u64 = 10 * 1024 * 1024;
/// A frame's canvas counts even when its encoded update covers only a small rectangle.
/// At most 100 megapixels across all frames, checked before evaluating any pixels.
pub const MAX_ANIMATION_PIXELS: u64 = 100_000_000;
pub const MAX_LEGACY_PIXELS: u64 = 100_000_000;
pub const ANIMATED_KEY: &str = "branding_animated";
/// Retain the purpose after replacement detaches a blob whose analysis is still queued.
pub const MARK_KEY: &str = "branding_mark";

/// Includes waiting for the branding slot, header probing and pixel evaluation.
pub fn processing_timeout(_blob: &Blob) -> Duration {
    #[cfg(feature = "test-support")]
    if test_hooks::held(&_blob.key).is_some() {
        return Duration::from_millis(500);
    }
    Duration::from_secs(10)
}

fn configure_cancellation(_blob: &Blob, _cancel: &Cancellation) {
    #[cfg(feature = "test-support")]
    if let Some((phase, reached, released)) = test_hooks::held(&_blob.key) {
        match released {
            Some(released) => _cancel.block_reads(reached, released),
            None => _cancel.stall_at(phase, reached),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Kind {
    Logo,
    Banner,
    Emoji,
}

impl Kind {
    pub fn maximum_dimensions(self) -> (i32, i32) {
        match self {
            Self::Logo => (4096, 4096),
            Self::Banner => (4096, 2304),
            Self::Emoji => (512, 512),
        }
    }

    pub fn dimension_message(self) -> &'static str {
        match self {
            Self::Logo => "must be at most 4096 × 4096 pixels",
            Self::Banner => "must be at most 4096 × 2304 pixels",
            Self::Emoji => "must be at most 512 × 512 pixels",
        }
    }

    pub fn still_variation(self) -> Variation {
        let (width, height) = match self {
            Self::Logo => (512, 512),
            Self::Banner => (1920, 1080),
            Self::Emoji => (128, 128),
        };
        Variation::resize_to_limit(width, height, Some("png"))
    }

    fn animated_key(self) -> &'static str {
        match self {
            Self::Emoji => crate::workspace_icon::ANIMATED_KEY,
            Self::Logo | Self::Banner => ANIMATED_KEY,
        }
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
            Self::Format if matches!(kind, Kind::Emoji) => "must be an SVG, PNG, GIF or WebP image",
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

/// Must run on the branding blocking pool. The strict, single-page loader only reads headers
/// until dimensions and the total animation budget have been checked.
pub fn prepare(
    storage: &Storage,
    blob: Blob,
    kind: Kind,
    cancel: &Cancellation,
) -> Result<Prepared, Invalid> {
    prepare_with_limit(storage, blob, kind, cancel, MAX_BYTES)
}

/// Use the same decoded image budgets with a caller's upload byte limit.
pub fn prepare_with_limit(
    storage: &Storage,
    mut blob: Blob,
    kind: Kind,
    cancel: &Cancellation,
    max_bytes: u64,
) -> Result<Prepared, Invalid> {
    if blob.byte_size < 0 || blob.byte_size as u64 > max_bytes {
        return Err(Invalid::Size);
    }
    configure_cancellation(&blob, cancel);
    let path = storage.service.path_for(&blob.key);
    let (image, content_type) = header(&path, kind, cancel, max_bytes)?;
    blob.content_type = Some(content_type.to_owned());
    blob.metadata.set("identified", Json::Bool(true));
    blob.metadata
        .merge(&crate::analyze::image_dimensions(&image));
    blob.metadata.set("analyzed", Json::Bool(true));
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
        transform(storage, &blob.filename, image, &variation, cancel)
            .ok()
            .map(|staged| (variation, staged))
    } else {
        None
    };
    blob.metadata.set(ANIMATED_KEY, Json::Bool(still.is_some()));
    Ok(Prepared { blob, still })
}

pub struct PreparedEmoji {
    pub content_type: &'static str,
    pub metadata: Json,
    pub still: Option<(Variation, Staged)>,
}

/// GIF/WebP emoji has a smaller byte/canvas/frame budget and must decode completely before acceptance.
pub(crate) fn prepare_emoji(
    storage: &Storage,
    key: &str,
    filename: &Filename,
    cancel: &Cancellation,
) -> Result<PreparedEmoji, String> {
    let path = storage.service.path_for(key);
    let (image, content_type) = header(&path, Kind::Emoji, cancel, MAX_BYTES)
        .map_err(|invalid| invalid.message(Kind::Emoji).to_owned())?;
    if image.width() != image.height() {
        return Err("must be square".into());
    }
    if image.width() < 64 {
        return Err("must be at least 64 pixels wide and tall".into());
    }
    let pages = if matches!(content_type, "image/gif" | "image/webp") {
        image
            .get_string("n-pages")
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(1)
    } else {
        1
    };
    if pages > 100 {
        return Err("must contain at most 100 frames".into());
    }
    let decoded =
        Image::open_emoji(&path, content_type, -1, cancel).map_err(|_| "could not be read")?;
    decoded.average().map_err(|_| "could not be read")?;
    cancel.check().map_err(|_| "could not be read")?;
    let still = if pages > 1 {
        let variation = Kind::Emoji.still_variation();
        let staged = transform(storage, filename, image, &variation, cancel)
            .map_err(|_| "could not be read")?;
        Some((variation, staged))
    } else {
        None
    };
    let mut metadata = crate::analyze::image_dimensions(&decoded);
    // The all-page decoder's height is stacked; publish the canvas height instead.
    metadata.set(
        "height",
        Json::Int(
            decoded
                .get_string("page-height")
                .and_then(|n| n.parse().ok())
                .unwrap_or(decoded.height().into()),
        ),
    );
    metadata.set("identified", Json::Bool(true));
    metadata.set("analyzed", Json::Bool(true));
    metadata.set(
        crate::workspace_icon::ANIMATED_KEY,
        Json::Bool(still.is_some()),
    );
    Ok(PreparedEmoji {
        content_type,
        metadata,
        still,
    })
}

/// Legacy branding analysis reads bounded headers directly, without `Storage::open`'s copy.
/// It keeps legacy animation semantics: only a validated upload opts into animation.
pub fn analyzed_metadata(
    storage: &Storage,
    blob: &Blob,
    cancel: &Cancellation,
) -> crate::Result<Json> {
    configure_cancellation(blob, cancel);
    cancel.check()?;
    let mut metadata = if blob.content_type().starts_with("image") {
        crate::analyze::image_dimensions(&legacy_image(storage, blob, cancel)?)
    } else {
        Json::object()
    };
    cancel.check()?;
    metadata.set("analyzed", Json::Bool(true));
    Ok(metadata)
}

/// Older/classic attachments retain the classic variable formats and first-page conversion,
/// with a larger pixel budget. API uploads retain their strict validation on cache misses.
pub fn transform_variant(
    storage: &Storage,
    blob: &Blob,
    kind: Kind,
    variation: &Variation,
    cancel: &Cancellation,
) -> crate::Result<Staged> {
    configure_cancellation(blob, cancel);
    let path = storage.service.path_for(&blob.key);
    let image = if blob.metadata.get(kind.animated_key()).is_none() {
        if !blob.is_variable() {
            return Err(crate::Error::Invariable(blob.content_type().to_owned()));
        }
        legacy_image(storage, blob, cancel)?
    } else {
        header(&path, kind, cancel, MAX_BYTES)
            .map_err(|invalid| crate::Error::Analyze(invalid.message(kind).to_owned()))?
            .0
    };
    transform(storage, &blob.filename, image, variation, cancel)
}

fn legacy_image(storage: &Storage, blob: &Blob, cancel: &Cancellation) -> crate::Result<Image> {
    cancel.check()?;
    let path = storage.service.path_for(&blob.key);
    if blob.byte_size > MAX_BYTES as i64 || std::fs::metadata(&path)?.len() > MAX_BYTES {
        return Err(crate::Error::Analyze(
            "legacy image exceeds the 10 MB byte budget".into(),
        ));
    }
    let image = Image::open_legacy_branding(&path, cancel)?;
    let (width, height) = (image.width(), image.height());
    if width <= 0 || height <= 0 || width as u64 * height as u64 > MAX_LEGACY_PIXELS {
        return Err(crate::Error::Analyze(
            "legacy image exceeds the 100 megapixel budget".into(),
        ));
    }
    Ok(image)
}

fn header(
    path: &Path,
    kind: Kind,
    cancel: &Cancellation,
    max_bytes: u64,
) -> Result<(Image, &'static str), Invalid> {
    cancel.check().map_err(|_| Invalid::Format)?;
    let mut file = std::fs::File::open(path).map_err(|_| Invalid::Format)?;
    if file.metadata().map_err(|_| Invalid::Format)?.len() > max_bytes {
        return Err(Invalid::Size);
    }
    let mut magic = [0; 12];
    file.read_exact(&mut magic).map_err(|_| Invalid::Format)?;
    let content_type = if magic.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if magic.starts_with(b"\xff\xd8\xff") && !matches!(kind, Kind::Emoji) {
        "image/jpeg"
    } else if magic.starts_with(b"GIF87a") || magic.starts_with(b"GIF89a") {
        "image/gif"
    } else if magic.starts_with(b"RIFF") && &magic[8..] == b"WEBP" {
        "image/webp"
    } else {
        return Err(Invalid::Format);
    };
    let image = if matches!(kind, Kind::Emoji) {
        Image::open_emoji(path, content_type, 1, cancel)
    } else {
        Image::open_branding(path, content_type, cancel)
    }
    .map_err(|_| Invalid::Format)?;
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
    filename: &Filename,
    image: Image,
    variation: &Variation,
    cancel: &Cancellation,
) -> crate::Result<Staged> {
    cancel.check()?;
    let format = variation.format()?;
    let image = image.autorot()?;
    image.cancel_on(cancel)?;
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
    image.cancel_on(cancel)?;
    let output = tempfile::Builder::new()
        .prefix("branding-")
        .suffix(&format!(".{format}"))
        .tempfile()?;
    image.write_to_file(output.path())?;
    cancel.check()?;
    // The generated, bounded image already supplies its dimensions. Avoid starting another
    // uncancellable analyzer/header read while staging it.
    let staged = storage
        .stage_file(
            output.path(),
            Filename::new(format!("{}.{format}", filename.base())),
            Some(&variation.content_type()?),
        )?
        .with_image_dimensions(image.width(), image.height());
    cancel.check()?;
    Ok(staged)
}

#[cfg(feature = "test-support")]
pub mod test_hooks {
    use crate::vips::StallPhase;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, LazyLock, Mutex};

    type Hold = (StallPhase, Arc<AtomicBool>, Option<Arc<AtomicBool>>);
    static HELD: LazyLock<Mutex<HashMap<String, Hold>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    pub struct Stall {
        key: String,
        pub reached: Arc<AtomicBool>,
        released: Option<Arc<AtomicBool>>,
    }

    impl Drop for Stall {
        fn drop(&mut self) {
            if let Some(released) = &self.released {
                released.store(true, Ordering::Relaxed);
            }
            HELD.lock().unwrap().remove(&self.key);
        }
    }

    /// Stall inside the actual source/eval callback; only cancellation can end the work.
    pub fn stall(key: &str, phase: StallPhase) -> Stall {
        hold(key, phase, None)
    }

    /// Block a source read without checking cancellation, until the test drops this guard.
    pub fn block_reads(key: &str) -> Stall {
        hold(
            key,
            StallPhase::Header,
            Some(Arc::new(AtomicBool::new(false))),
        )
    }

    fn hold(key: &str, phase: StallPhase, released: Option<Arc<AtomicBool>>) -> Stall {
        let reached = Arc::new(AtomicBool::new(false));
        HELD.lock()
            .unwrap()
            .insert(key.to_owned(), (phase, reached.clone(), released.clone()));
        Stall {
            key: key.to_owned(),
            reached,
            released,
        }
    }

    pub(super) fn held(key: &str) -> Option<Hold> {
        HELD.lock().unwrap().get(key).cloned()
    }
}
