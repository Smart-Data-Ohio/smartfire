use campfire_api_types::MessageSticker;
use campfire_db::{Connection, Message, Result, models::sticker::Sticker};
use campfire_storage::{Storage, branding::Kind, paths};

use super::attachments;

pub enum StickerUrls<'a> {
    Session,
    Signed { storage: &'a Storage, base: &'a str },
}

pub fn message_sticker(
    conn: &Connection,
    message: &Message,
    urls: StickerUrls<'_>,
) -> Result<Option<MessageSticker>> {
    let sticker = match message
        .sticker_id
        .map(|id| Sticker::find(conn, id))
        .transpose()
    {
        Ok(Some(sticker)) => sticker,
        Ok(None) | Err(campfire_db::Error::RecordNotFound("Sticker")) => {
            return Ok(message
                .sticker_name
                .clone()
                .map(|name| MessageSticker::Deleted { name }));
        }
        Err(error) => return Err(error),
    };
    let blob = attachments::attached_blob(conn, "Sticker", sticker.id, "image")?
        .ok_or(campfire_db::Error::RecordNotFound("sticker image"))?;
    let animated = campfire_storage::workspace_icon::animated(&blob);
    let (image_url, still_url) = match urls {
        StickerUrls::Session => {
            let image = format!("/stickers/{}", sticker.id);
            let still = if animated {
                format!("{image}?still=1")
            } else {
                image.clone()
            };
            (image, still)
        }
        StickerUrls::Signed { storage, base } => {
            let image = format!(
                "{base}{}",
                paths::blob_redirect_path(&*storage.verifier, &blob, Some("inline"))
            );
            let still = if animated {
                format!(
                    "{base}{}",
                    paths::representation_redirect_path(
                        &*storage.verifier,
                        &blob,
                        &Kind::Sticker.still_variation()
                    )
                )
            } else {
                image.clone()
            };
            (image, still)
        }
    };
    Ok(Some(MessageSticker::Available {
        id: sticker.id,
        name: sticker.name,
        image_url,
        still_url,
        animated,
    }))
}

pub fn webhook_payload(
    conn: &Connection,
    storage: &Storage,
    message: &Message,
    base: &str,
    body: String,
) -> Result<String> {
    let Some(sticker) = message_sticker(conn, message, StickerUrls::Signed { storage, base })?
    else {
        return Ok(body);
    };
    let mut payload: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
    payload["message"]["sticker"] = serde_json::json!(sticker);
    Ok(campfire_presentation::helpers::to_rails_json(&payload))
}
