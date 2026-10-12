//! The composer's GIF picker, backed by KLIPY, proxied by this server: search,
//! trending and categories (`campfire_app::integrations::klipy`), picker previews
//! through a media proxy limited to KLIPY's media hosts, and sending, which
//! downloads the chosen GIF and stores it as a normal message attachment (see
//! `CreateMessage.gifSlug`). Browsers never talk to KLIPY — their addresses and
//! the API key stay here — and a sent GIF never hotlinks KLIPY.

use axum::{Router, routing::get};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_app::integrations::klipy::{self, Item, MediaError};
use campfire_db::CachedStatements;
use campfire_kit::{Ctx, Error, Kit, Result, SendOptions, StatusCode, action};
use campfire_storage::Filename;

use crate::admin::viewer;
use crate::error::{fail, validation};

/// Search/trending/categories calls one user may make per minute.
const API_LIMIT: usize = 30;
/// Preview fetches one user may make per minute (a picker page shows many).
const MEDIA_LIMIT: usize = 240;
/// Picker previews are small renditions; anything bigger is not a preview.
const MEDIA_MAX_BYTES: usize = 5 * 1024 * 1024;

pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/gifs/config", get(action(config)))
        .route("/api/v1/gifs/search", get(action(search)))
        .route("/api/v1/gifs/trending", get(action(trending)))
        .route("/api/v1/gifs/categories", get(action(categories)))
        .route("/api/v1/gifs/media", get(action(media)))
}

endpoint!(config => show_config);
endpoint!(search => search_gifs);
endpoint!(trending => trending_gifs);
endpoint!(categories => list_categories);
endpoint!(media => proxy_media);

fn unavailable(c: &mut Ctx) -> Error {
    fail(
        c,
        api::ApiError::Unavailable {
            message: "GIFs are unavailable right now".into(),
        },
    )
}

/// The configured key, or the typed unavailable error when GIFs are off.
fn key(c: &mut Ctx) -> Result<String> {
    c.app()
        .config
        .klipy_api_key
        .clone()
        .ok_or_else(|| unavailable(c))
}

fn customer(c: &Ctx, user_id: i64) -> String {
    klipy::customer_id(&c.app().config.secret_key_base, user_id)
}

fn limited(c: &mut Ctx, user_id: i64, bucket: &'static str, limit: usize) -> Result<()> {
    if c.app().klipy.allow(user_id, bucket, limit) {
        return Ok(());
    }
    Err(fail(
        c,
        api::ApiError::RateLimited {
            message: "Too many GIF requests; try again in a minute".into(),
            retry_after: 60,
        },
    ))
}

fn page(c: &Ctx) -> u32 {
    c.param_str("page")
        .and_then(|page| page.parse().ok())
        .filter(|page| *page >= 1)
        .unwrap_or(1)
}

fn item_dto(item: &Item) -> Option<api::GifItem> {
    let preview = item.preview()?;
    let still = item.still().unwrap_or(preview);
    Some(api::GifItem {
        slug: item.slug.clone(),
        klipy_id: item.klipy_id.clone(),
        title: item.title.clone(),
        preview_url: klipy::media_proxy_path(&preview.url),
        still_url: klipy::media_proxy_path(&still.url),
        width: preview.width,
        height: preview.height,
    })
}

fn list_reply(c: &mut Ctx, page: &klipy::ListPage) -> Result {
    c.json(
        StatusCode::OK,
        &api::GifList {
            items: page.items.iter().filter_map(item_dto).collect(),
            page: page.page,
            has_next: page.has_next,
        },
    )
}

async fn show_config(c: &mut Ctx) -> Result {
    viewer(c).await?;
    c.json(
        StatusCode::OK,
        &api::GifConfig {
            enabled: c.app().config.klipy_api_key.is_some(),
        },
    )
}

async fn search_gifs(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let key = key(c)?;
    let query = c.param_str("q").unwrap_or_default().trim().to_string();
    if query.is_empty() {
        return Err(fail(c, validation("q", "can't be blank")));
    }
    limited(c, user.id, "api", API_LIMIT)?;
    let filter = c.app().config.klipy_content_filter.clone();
    let page = c
        .app()
        .klipy
        .search(&key, &filter, &customer(c, user.id), &query, page(c))
        .await
        .map_err(|_| unavailable(c))?;
    list_reply(c, &page)
}

async fn trending_gifs(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let key = key(c)?;
    limited(c, user.id, "api", API_LIMIT)?;
    let filter = c.app().config.klipy_content_filter.clone();
    let page = c
        .app()
        .klipy
        .trending(&key, &filter, &customer(c, user.id), page(c))
        .await
        .map_err(|_| unavailable(c))?;
    list_reply(c, &page)
}

async fn list_categories(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let key = key(c)?;
    limited(c, user.id, "api", API_LIMIT)?;
    let categories = c
        .app()
        .klipy
        .categories(&key)
        .await
        .map_err(|_| unavailable(c))?;
    c.json(
        StatusCode::OK,
        &api::GifCategoryList {
            categories: categories
                .into_iter()
                .map(|category| api::GifCategory {
                    name: category.name,
                    query: category.query,
                    preview_url: category.preview_url.map(|url| klipy::media_proxy_path(&url)),
                })
                .collect(),
        },
    )
}

async fn proxy_media(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    key(c)?;
    let Some(url) = c.param_str("u").map(str::to_owned) else {
        return Err(fail(c, validation("u", "is missing")));
    };
    limited(c, user.id, "media", MEDIA_LIMIT)?;
    match c.app().klipy.fetch_media(&url, MEDIA_MAX_BYTES).await {
        Ok((bytes, content_type)) => {
            c.set_header("x-content-type-options", "nosniff");
            Ok(c.send_data(bytes, SendOptions::inline(&content_type)))
        }
        Err(MediaError::NotAllowed) => Err(fail(
            c,
            validation("u", "isn't a KLIPY media URL"),
        )),
        Err(MediaError::BadType) => Err(fail(
            c,
            validation("u", "isn't an image or video"),
        )),
        Err(MediaError::TooLarge | MediaError::Unavailable) => Err(unavailable(c)),
    }
}

/// Downloads the GIF behind `slug` and stores it as an uploaded blob owned by
/// `user_id`, returning its signed id for the normal message-attachment path.
/// The blob's metadata keeps the KLIPY item id and title, and its filename is
/// the title, so the attachment is described in the Files pane and search.
pub(crate) async fn stage_gif(c: &mut Ctx, user_id: i64, slug: &str) -> Result<String> {
    let key = key(c)?;
    limited(c, user_id, "api", API_LIMIT)?;
    let item = c
        .app()
        .klipy
        .item(&key, slug)
        .await
        .map_err(|_| unavailable(c))?;
    let cap = campfire_runtime::active_storage::upload_limit_bytes(c.app()).await?;
    let Some((rendition, format, ext)) = item.send_rendition(cap) else {
        return Err(fail(c, validation("gifSlug", "is too large to attach")));
    };
    let (rendition_url, format, ext) = (rendition.url.clone(), format, ext);
    let limit = usize::try_from(cap.max(0)).unwrap_or(0);
    let (bytes, _content_type) = c
        .app()
        .klipy
        .fetch_media(&rendition_url, limit)
        .await
        .map_err(|_| unavailable(c))?;
    let declared = match format {
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        _ => "image/gif",
    };
    let filename = Filename::new(format!("{}.{}", filename_stem(&item), ext));
    let storage = c.app().storage.clone();
    let staged = tokio::task::spawn_blocking(move || storage.stage_bytes(&bytes, filename, Some(declared)))
        .await
        .map_err(Error::internal)?
        .map_err(Error::internal)?;
    let (klipy_id, title) = (item.klipy_id.clone(), item.title.clone());
    let blob = c
        .app()
        .db
        .write(move |tx| {
            let mut blob = staged
                .insert(tx.conn(), tx.now().jiff())
                .map_err(campfire_runtime::presenters::attachments::storage_error)?;
            blob.metadata
                .set("uploader_id", campfire_storage::Json::Int(user_id));
            blob.metadata
                .set("klipy_id", campfire_storage::Json::String(klipy_id));
            blob.metadata
                .set("klipy_title", campfire_storage::Json::String(title));
            tx.conn().execute_cached(
                "UPDATE active_storage_blobs SET metadata=? WHERE id=?",
                rusqlite::params![blob.metadata.encode(), blob.id],
            )?;
            campfire_runtime::active_storage::keep_after_commit(tx, staged);
            Ok(blob)
        })
        .await
        .map_err(Error::internal)?;
    Ok(campfire_storage::paths::signed_blob_id(
        &*c.app().storage.verifier,
        blob.id,
        None,
    ))
}

/// A filesystem-safe stem from the GIF's title (falling back to its slug).
fn filename_stem(item: &Item) -> String {
    let title: String = item
        .title
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character == '-' {
                character
            } else {
                ' '
            }
        })
        .collect();
    let stem = title.split_whitespace().collect::<Vec<_>>().join("-");
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        item.slug.clone()
    } else {
        stem.chars().take(80).collect()
    }
}
