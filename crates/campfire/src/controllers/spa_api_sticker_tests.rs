//! Catalog, validation, private media and message references through the real sticker API.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::admin_tests::{app, error, get, parse, write};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER_KEY, Browser, DAVID, KEVIN, Req, TestApp,
};

const CATALOG: &str = "/api/v1/stickers";
const ADMIN: &str = "/api/v1/admin/stickers";
const PNG: &[u8] = include_bytes!("../../../../test-support/attachments/stickers/static.png");
const GIF: &[u8] = include_bytes!("../../../../test-support/attachments/stickers/animated.gif");
const APNG: &[u8] = include_bytes!("../../../../test-support/attachments/stickers/animated.png");
const HIDDEN: &[u8] =
    include_bytes!("../../../../test-support/attachments/stickers/hidden-default.png");

async fn upload(a: &TestApp, bytes: &[u8]) -> String {
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            bytes,
            campfire_storage::Filename::new("disguised.txt"),
            Some("text/plain"),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None)
}

async fn create(b: &mut Browser<'_>, name: &str, bytes: &[u8]) -> api::Sticker {
    let reply = b
        .write(Req::new(Method::POST, ADMIN).multipart(
            &[
                ("name", name),
                ("description", "Greeting"),
                ("tags", "👋 hello"),
            ],
            ("image", "disguised.txt", "text/plain", bytes),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let catalog: api::StickerCatalog = parse(&reply);
    catalog
        .stickers
        .into_iter()
        .find(|sticker| sticker.name == name.trim())
        .unwrap()
}

#[tokio::test]
async fn stickers_multipart_catalog_and_private_original_still_media() {
    let a = app().await.expect("restored default seed");
    let mut b = a.sign_in(DAVID).await;
    for (name, bytes, content_type, animated) in [
        ("Static", PNG, "image/png", false),
        ("GIF", GIF, "image/gif", true),
        ("APNG", APNG, "image/png", true),
        ("Hidden", HIDDEN, "image/png", true),
    ] {
        let sticker = create(&mut b, name, bytes).await;
        assert_eq!(sticker.animated, animated);
        assert_eq!(sticker.description, "Greeting");
        assert_eq!(sticker.tags, "👋 hello");
        let original = b.get(&sticker.image_url).await;
        assert_eq!(original.status, StatusCode::OK);
        assert_eq!(original.content_type(), Some(content_type));
        assert_eq!(original.body, bytes);
        let still = b.get(&sticker.still_url).await;
        assert_eq!(still.status, StatusCode::OK, "{}", still.text());
        assert_eq!(still.content_type(), Some("image/png"));
        assert_eq!(
            u32::from_be_bytes(still.body[16..20].try_into().unwrap()),
            2
        );
        assert_eq!(
            u32::from_be_bytes(still.body[20..24].try_into().unwrap()),
            3
        );
        if animated {
            let mut reader = png_reader(&still.body);
            let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
            let output = reader.next_frame(&mut buffer).unwrap();
            assert!(
                buffer[..output.buffer_size()]
                    .chunks_exact(output.color_type.samples())
                    .all(|pixel| pixel[..3] == [0, 0, 0]),
                "the first frame must be black, not the excluded red default or white second frame"
            );
            assert_ne!(original.header("etag"), still.header("etag"));
        } else {
            assert_eq!(sticker.image_url, sticker.still_url);
        }
        assert_eq!(
            b.send(get(&sticker.still_url).header("if-none-match", still.header("etag").unwrap()))
                .await
                .status,
            StatusCode::NOT_MODIFIED
        );
        let mut member = a.sign_in(KEVIN).await;
        assert_eq!(member.get(&sticker.still_url).await.status, StatusCode::OK);
        let mut guest = a.anonymous();
        assert_eq!(
            guest.get(&sticker.image_url).await.status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            guest.get(&sticker.still_url).await.status,
            StatusCode::NOT_FOUND
        );
    }
    let catalog: api::StickerCatalog = parse(&b.send(get(CATALOG)).await);
    assert_eq!(catalog.capacity, 60);
    assert_eq!(catalog.stickers.len(), 4);
}

fn png_reader(bytes: &[u8]) -> png::Reader<std::io::Cursor<&[u8]>> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    decoder.read_info().unwrap()
}

#[tokio::test]
async fn stickers_validate_animation_frames_and_exact_byte_name_limits() {
    let a = app().await.expect("restored default seed");
    let mut b = a.sign_in(DAVID).await;
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 2, 3);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_animated(101, 0).unwrap();
    let mut writer = encoder.write_header().unwrap();
    for _ in 0..101 {
        writer.write_image_data(&[0; 24]).unwrap();
    }
    writer.finish().unwrap();
    let signed = upload(&a, &bytes).await;
    let refused = write(
        &mut b,
        Method::POST,
        ADMIN,
        json!({"name":"Frames","signedId":signed}),
    )
    .await;
    assert_eq!(
        error(&refused)["fields"]["image"],
        json!(["must contain at most 100 frames"])
    );
    let mut frames = GIF[..19].to_vec();
    for _ in 0..101 {
        frames.extend_from_slice(&GIF[19..45]);
    }
    frames.push(b';');
    let signed = upload(&a, &frames).await;
    let refused = write(
        &mut b,
        Method::POST,
        ADMIN,
        json!({"name":"Frames","signedId":signed}),
    )
    .await;
    assert_eq!(
        error(&refused)["fields"]["image"],
        json!(["must contain at most 100 frames"])
    );
    let mut exact = PNG.to_vec();
    exact.resize(512 * 1024, 0);
    create(&mut b, "Hi", &exact).await;
    create(&mut b, &"👋".repeat(30), PNG).await;
}

#[tokio::test]
async fn stickers_post_as_first_and_later_thread_replies() {
    let a = app().await.expect("restored default seed");
    let mut b = a.sign_in(DAVID).await;
    let sticker = create(&mut b, "Hello", APNG).await;
    let root = write(
        &mut b,
        Method::POST,
        &format!("/api/v1/rooms/{ALL_TALK}/messages"),
        json!({"clientMessageId":"thread-parent","markdownSource":"Thread"}),
    )
    .await;
    let root: api::MessageDTO = parse(&root);
    let thread = write(&mut b, Method::POST, &format!("/api/v1/rooms/{ALL_TALK}/threads"), json!({"parentMessageId":root.id,"name":"Sticker thread","message":{"clientMessageId":"sticker-first","stickerId":sticker.id}})).await;
    assert_eq!(thread.status, StatusCode::CREATED, "{}", thread.text());
    let thread: api::ThreadCreated = parse(&thread);
    assert!(matches!(
        thread.message.sticker,
        Some(api::MessageSticker::Available { animated: true, .. })
    ));
    let reply = write(
        &mut b,
        Method::POST,
        &format!("/api/v1/threads/{}/messages", thread.detail.thread.id),
        json!({"clientMessageId":"sticker-later","stickerId":sticker.id}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let reply: api::MessageDTO = parse(&reply);
    assert_eq!(reply.sticker, thread.message.sticker);
}

#[tokio::test]
async fn stickers_validate_every_image_bound_and_ignore_upload_labels() {
    let a = app().await.expect("restored default seed");
    let mut b = a.sign_in(DAVID).await;
    let large = vec![0; 512 * 1024 + 1];
    for (bytes, message) in [
        (large.as_slice(), "must be 512 KB or smaller"),
        (
            include_bytes!("../../../../test-support/attachments/stickers/wide.png").as_slice(),
            "must be at most 320 × 320 pixels",
        ),
        (
            include_bytes!("../../../../test-support/attachments/stickers/tall.png").as_slice(),
            "must be at most 320 × 320 pixels",
        ),
        (
            b"<svg xmlns='http://www.w3.org/2000/svg'/>".as_slice(),
            "must be a PNG, APNG or GIF image",
        ),
        (
            include_bytes!("../../../../fixtures/files/workspace_icons/animated.webp").as_slice(),
            "must be a PNG, APNG or GIF image",
        ),
        (
            b"not an image".as_slice(),
            "must be a PNG, APNG or GIF image",
        ),
        (&GIF[..GIF.len() - 1], "could not be read"),
        (&APNG[..APNG.len() - 24], "could not be read"),
        (&PNG[..PNG.len() - 24], "could not be read"),
    ] {
        let signed = upload(&a, bytes).await;
        let reply = write(
            &mut b,
            Method::POST,
            ADMIN,
            json!({"name":"Invalid", "signedId":signed}),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(error(&reply)["fields"]["image"], json!([message]));
    }
    let mut huge_canvas = GIF.to_vec();
    huge_canvas[6..10].copy_from_slice(&[0xff; 4]);
    let signed = upload(&a, &huge_canvas).await;
    let reply = write(
        &mut b,
        Method::POST,
        ADMIN,
        json!({"name":"Huge", "signedId":signed}),
    )
    .await;
    assert_eq!(
        error(&reply)["fields"]["image"],
        json!(["must be at most 320 × 320 pixels"])
    );
    let missing = write(
        &mut b,
        Method::POST,
        ADMIN,
        json!({"name":"Missing","signedId":"invalid"}),
    )
    .await;
    assert_eq!(
        error(&missing)["fields"]["image"],
        json!(["isn't an uploaded file"])
    );
    assert!(
        parse::<api::StickerCatalog>(&b.send(get(CATALOG)).await)
            .stickers
            .is_empty()
    );
    create(
        &mut b,
        "Maximum",
        include_bytes!("../../../../test-support/attachments/stickers/maximum.png"),
    )
    .await;
}

#[tokio::test]
async fn stickers_names_permissions_and_catalog_edits() {
    let a = app().await.expect("restored default seed");
    let mut b = a.sign_in(DAVID).await;
    let signed = upload(&a, PNG).await;
    for name in ["", " ", "x", &"a".repeat(31), &"👋".repeat(31)] {
        let reply = write(
            &mut b,
            Method::POST,
            ADMIN,
            json!({"name":name,"signedId":signed}),
        )
        .await;
        assert_eq!(
            error(&reply)["fields"]["name"],
            json!(["must be between 2 and 30 characters"])
        );
    }
    let sticker = create(&mut b, "  Straße  ", PNG).await;
    let duplicate = write(
        &mut b,
        Method::POST,
        ADMIN,
        json!({"name":"STRASSE","signedId":signed}),
    )
    .await;
    assert_eq!(
        error(&duplicate)["fields"]["name"],
        json!(["has already been taken"])
    );
    let path = format!("{ADMIN}/{}", sticker.id);
    let mut member = a.sign_in(KEVIN).await;
    assert_eq!(member.send(get(CATALOG)).await.status, StatusCode::OK);
    assert_eq!(member.send(get(ADMIN)).await.status, StatusCode::FORBIDDEN);
    for (method, path) in [
        (Method::POST, ADMIN),
        (Method::PATCH, path.as_str()),
        (Method::DELETE, path.as_str()),
    ] {
        assert_eq!(
            write(
                &mut member,
                method,
                path,
                json!({"name":"Wave","signedId":signed})
            )
            .await
            .status,
            StatusCode::FORBIDDEN
        );
    }
    let mut guest = a.anonymous();
    assert_eq!(
        guest.send(get(CATALOG)).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        b.send(Req::new(Method::DELETE, &path).header("x-csrf-token", "invalid"))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let edited = write(
        &mut b,
        Method::PATCH,
        &path,
        json!({"name":"Wave","description":"Goodbye","tags":"👋 farewell"}),
    )
    .await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.text());
    let edited: api::StickerCatalog = parse(&edited);
    assert_eq!(edited.stickers[0].description, "Goodbye");
    assert_eq!(edited.stickers[0].tags, "👋 farewell");
    assert_eq!(
        write(&mut b, Method::DELETE, &path, Value::Null)
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        b.get(&sticker.image_url).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        write(
            &mut b,
            Method::PATCH,
            &path,
            json!({"name":"Wave","description":"","tags":""})
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn stickers_capacity_is_atomic_and_has_a_typed_limit_error() {
    let a = app().await.expect("restored default seed");
    let mut b = a.sign_in(DAVID).await;
    let signed = upload(&a, PNG).await;
    for index in 0..59 {
        let reply = write(
            &mut b,
            Method::POST,
            ADMIN,
            json!({"name":format!("Sticker {index}"),"signedId":signed}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    }
    let mut second = a.sign_in(DAVID).await;
    let (first, second) = tokio::join!(
        write(
            &mut b,
            Method::POST,
            ADMIN,
            json!({"name":"Last one","signedId":signed})
        ),
        write(
            &mut second,
            Method::POST,
            ADMIN,
            json!({"name":"Last two","signedId":signed})
        ),
    );
    let mut statuses = [first.status.as_u16(), second.status.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [201, 422]);
    let refused = if first.status == StatusCode::UNPROCESSABLE_ENTITY {
        first
    } else {
        second
    };
    assert_eq!(error(&refused)["_tag"], "StickerLimitReached");
    assert_eq!(error(&refused)["capacity"], 60);
    let catalog: api::StickerCatalog = parse(&b.send(get(CATALOG)).await);
    assert_eq!(catalog.stickers.len(), 60);
    write(
        &mut b,
        Method::DELETE,
        &format!("{ADMIN}/{}", catalog.stickers[0].id),
        Value::Null,
    )
    .await;
    assert_eq!(
        write(
            &mut b,
            Method::POST,
            ADMIN,
            json!({"name":"Replacement","signedId":signed})
        )
        .await
        .status,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn sticker_messages_sync_search_notifications_and_deleted_placeholders() {
    let a = app().await.expect("restored default seed");
    let mut b = a.sign_in(DAVID).await;
    let sticker = create(&mut b, "Hello", GIF).await;
    let (mut sync, cable) =
        crate::controllers::messages::attachment_processing_tests::json_subscribe(&a).await;
    let messages = format!("/api/v1/rooms/{ALL_TALK}/messages");
    let invalid = write(
        &mut b,
        Method::POST,
        &messages,
        json!({"clientMessageId":"invalid-sticker","stickerId":-1}),
    )
    .await;
    assert_eq!(
        error(&invalid)["fields"]["stickerId"],
        json!(["must exist"])
    );
    let mut created_id = 0;
    for (client, markdown) in [("sticker-only", ""), ("sticker-text", "Welcome!")] {
        let reply = write(
            &mut b,
            Method::POST,
            &messages,
            json!({"clientMessageId":client,"markdownSource":markdown,"stickerId":sticker.id}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let created: api::MessageDTO = parse(&reply);
        created_id = created.id;
        assert!(matches!(
            created.sticker,
            Some(api::MessageSticker::Available { animated: true, .. })
        ));
        let event = sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == created.id), |_| false).await;
        let api::SyncPayload::MessageCreated(message) = event.payload else {
            panic!("message.created")
        };
        assert_eq!(message.sticker, created.sticker);
        let id = created.id;
        let rich_text = a.db().env().rich_text.clone();
        let plain = a
            .db()
            .read(move |conn| {
                let message = campfire_db::Message::find(conn, id)?;
                message.notification_text(conn, &*rich_text)
            })
            .await
            .unwrap();
        assert_eq!(
            plain,
            if markdown.is_empty() {
                "Sticker: Hello"
            } else {
                markdown
            }
        );
    }
    let renamed = write(
        &mut b,
        Method::PATCH,
        &format!("{ADMIN}/{}", sticker.id),
        json!({"name":"Wave","description":"","tags":""}),
    )
    .await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.text());
    let deleted = write(
        &mut b,
        Method::DELETE,
        &format!("{ADMIN}/{}", sticker.id),
        Value::Null,
    )
    .await;
    assert_eq!(deleted.status, StatusCode::OK);
    let event = sync.until(|event| matches!(&event.payload, api::SyncPayload::MessageUpdated(message) if message.id == created_id && matches!(message.sticker, Some(api::MessageSticker::Deleted { .. }))), |_| false).await;
    let api::SyncPayload::MessageUpdated(message) = event.payload else {
        panic!("message.updated")
    };
    assert_eq!(
        message.sticker,
        Some(api::MessageSticker::Deleted {
            name: "Wave".into()
        })
    );
    let page: api::MessagePage = parse(&b.send(get(&messages)).await);
    assert!(page.messages.iter().filter(|message| message.client_message_id.starts_with("sticker-")).all(|message| matches!(&message.sticker, Some(api::MessageSticker::Deleted { name }) if name == "Wave")));
    let again = write(
        &mut b,
        Method::POST,
        &messages,
        json!({"clientMessageId":"sticker-only","stickerId":sticker.id}),
    )
    .await;
    assert_eq!(again.status, StatusCode::OK);
    let old: api::MessageDTO = parse(&again);
    let edited = write(
        &mut b,
        Method::PATCH,
        &format!("/api/v1/messages/{}", old.id),
        json!({"markdownSource":""}),
    )
    .await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.text());
    let hits = a
        .db()
        .read(move |conn| campfire_db::Message::search_in_room(conn, ALL_TALK, "Wave"))
        .await
        .unwrap();
    assert!(hits.iter().any(|message| message.id == old.id));
    cable.abort();
}

async fn sticker_history_preserves_display_text(mcp: bool) {
    let a = crate::controllers::agent_http_tests::setup()
        .await
        .without_job_runner()
        .await;
    let mut b = a.sign_in(DAVID).await;
    let sticker = create(&mut b, "Hello", PNG).await;
    let reply = write(
        &mut b,
        Method::POST,
        &format!("/api/v1/rooms/{ALL_TALK}/messages"),
        json!({"clientMessageId":"history-sticker","markdownSource":"Caption","stickerId":sticker.id}),
    ).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let created: api::MessageDTO = parse(&reply);
    let edited = write(
        &mut b,
        Method::PATCH,
        &format!("/api/v1/messages/{}", created.id),
        json!({"markdownSource":""}),
    )
    .await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.text());
    let mut texts = Vec::new();
    for deleted in [false, true] {
        if deleted {
            assert_eq!(
                write(
                    &mut b,
                    Method::DELETE,
                    &format!("{ADMIN}/{}", sticker.id),
                    Value::Null
                )
                .await
                .status,
                StatusCode::OK
            );
        }
        let request = if mcp {
            Req::new(Method::POST, "/agents/mcp")
                .header("content-type", "application/json")
                .body(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_messages","arguments":{"room_id":ALL_TALK}}}).to_string())
        } else {
            Req::new(
                Method::GET,
                &format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages"),
            )
        };
        let request = if mcp {
            request.header(
                "authorization",
                &format!("Bearer {}", crate::controllers::agent_http_tests::SECRET),
            )
        } else {
            request
        };
        let reply = a
            .anonymous()
            .send(request.header("accept", "application/json"))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let payload = reply.json();
        let messages = if mcp {
            &payload["result"]["structuredContent"]["messages"]
        } else {
            &payload
        };
        let message = messages
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["id"] == created.id)
            .unwrap();
        texts.push(message["body"]["plain_text"].clone());
    }
    assert_eq!(
        texts,
        vec![json!("Sticker: Hello"), json!("Sticker: Hello")],
        "mcp={mcp}: live and deleted sticker text"
    );
}

#[tokio::test]
async fn sticker_agent_read_messages_preserves_blank_and_deleted_sticker_text() {
    sticker_history_preserves_display_text(true).await;
}

#[tokio::test]
async fn sticker_bot_history_preserves_blank_and_deleted_sticker_text() {
    sticker_history_preserves_display_text(false).await;
}
