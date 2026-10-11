//! Voice soundboard catalog permissions, storage and validation through the real API.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::admin_tests::{app, error, get, parse, write};
use crate::controllers::presenters::test_support::{Browser, DAVID, KEVIN, Req, TestApp};

const CATALOG: &str = "/api/v1/soundboard/sounds";
const ADMIN: &str = "/api/v1/admin/soundboard/sounds";
const WAV: &[u8] = include_bytes!("../../../../test-support/attachments/analysis.wav");

async fn upload(a: &TestApp, bytes: &[u8], filename: &str, content_type: &str) -> String {
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            bytes,
            campfire_storage::Filename::new(filename),
            Some(content_type),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None)
}

async fn legacy_upload(
    admin: &mut Browser<'_>,
    bytes: &[u8],
    content_type: &str,
    identified: Value,
) -> (i64, String) {
    let reply = write(
        admin,
        Method::POST,
        "/rails/active_storage/direct_uploads",
        json!({"blob": {
            "filename":"sound.mp3", "byte_size":bytes.len(),
            "checksum":campfire_storage::key::checksum(bytes), "content_type":content_type,
            "metadata":{"identified":identified}
        }}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let upload: Value = parse(&reply);
    let reply = admin
        .send(
            Req::new(
                Method::PUT,
                upload["direct_upload"]["url"].as_str().unwrap(),
            )
            .header("content-type", content_type)
            .header("content-length", &bytes.len().to_string())
            .body(bytes),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    (
        upload["id"].as_i64().unwrap(),
        upload["signed_id"].as_str().unwrap().to_owned(),
    )
}

#[tokio::test]
async fn soundboard_rejects_flac_with_forged_identified_metadata() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audio.flac");
    let result = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-threads",
            "1",
            "-filter_threads",
            "1",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=mono",
            "-t",
            "0.1",
            "-c:a",
            "flac",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.len() < 512 * 1024);
    let (_, signed_id) = legacy_upload(&mut admin, &bytes, "audio/mpeg", json!(true)).await;
    let reply = write(
        &mut admin,
        Method::POST,
        ADMIN,
        json!({"name":"Spoof","signedId":signed_id}),
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert!(error(&reply)["fields"]["audio"].is_array());
    assert_eq!(
        parse::<api::SoundboardCatalog>(&admin.send(get(CATALOG)).await).sounds,
        []
    );
}

#[tokio::test]
async fn soundboard_saves_detected_wav_type_despite_forged_upload_metadata() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    for (index, content_type, identified) in [
        (0, "audio/mpeg", json!(true)),
        (1, "audio/flac", json!("yes")),
        (2, "application/octet-stream", json!(1)),
    ] {
        let (id, signed_id) = legacy_upload(&mut admin, WAV, content_type, identified).await;
        let reply = write(
            &mut admin,
            Method::POST,
            ADMIN,
            json!({"name":format!("WAV {index}"),"signedId":signed_id}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let stored_type: String = a
            .db()
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT content_type FROM active_storage_blobs WHERE id=?",
                    [id],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(stored_type, "audio/wav");
        let catalog: api::SoundboardCatalog = parse(&reply);
        let fetched = admin.get(&catalog.sounds[index].audio_url).await;
        assert_eq!(fetched.status, StatusCode::OK);
        assert_eq!(fetched.headers["content-type"], "audio/wav");
        assert_eq!(fetched.body, WAV);
    }
}

#[tokio::test]
async fn soundboard_admin_crud_member_listing_and_attachment_cleanup() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let mut member = a.sign_in(KEVIN).await;
    let empty = member.send(get(CATALOG)).await;
    assert_eq!(empty.status, StatusCode::OK);
    assert_eq!(parse::<api::SoundboardCatalog>(&empty).sounds, []);
    let signed_id = upload(&a, WAV, "sound.wav", "audio/wav").await;
    let created = write(
        &mut admin,
        Method::POST,
        ADMIN,
        json!({"name":"  Bell  ","emoji":"🔔","signedId":signed_id}),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let catalog: api::SoundboardCatalog = parse(&created);
    assert_eq!(catalog.capacity, 48);
    assert_eq!(catalog.sounds.len(), 1);
    let sound = &catalog.sounds[0];
    assert_eq!(sound.name, "Bell");
    assert_eq!(sound.emoji.as_deref(), Some("🔔"));
    assert_eq!(sound.volume, 1.0);
    assert_eq!(sound.uploader_id, DAVID);
    assert_eq!(sound.created_at, sound.updated_at);
    let original = member.get(&sound.audio_url).await;
    assert_eq!(original.status, StatusCode::OK);
    assert_eq!(original.body, WAV);
    let list = member.send(get(CATALOG)).await;
    assert_eq!(list.status, StatusCode::OK);
    assert_eq!(parse::<api::SoundboardCatalog>(&list), catalog);
    assert_eq!(list.headers["cache-control"], "no-store");
    let id = sound.id;
    let path = format!("{ADMIN}/{id}");
    let changed = write(
        &mut admin,
        Method::PATCH,
        &path,
        json!({"name":"Chime","emoji":null,"volume":0.25}),
    )
    .await;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.text());
    let changed: api::SoundboardCatalog = parse(&changed);
    assert_eq!(changed.sounds[0].name, "Chime");
    assert_eq!(changed.sounds[0].emoji, None);
    assert_eq!(changed.sounds[0].volume, 0.25);
    assert_eq!(changed.sounds[0].uploader_id, DAVID);
    assert_eq!(changed.sounds[0].audio_url, sound.audio_url);
    let deleted = write(&mut admin, Method::DELETE, &path, Value::Null).await;
    assert_eq!(deleted.status, StatusCode::OK, "{}", deleted.text());
    assert_eq!(parse::<api::SoundboardCatalog>(&deleted).sounds, []);
    let (attachments, purges, audits) = a.db().read(move |conn| {
        Ok((
            conn.query_row("SELECT COUNT(*) FROM active_storage_attachments WHERE record_type='SoundboardSound' AND record_id=?", [id], |r| r.get::<_, i64>(0))?,
            conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='ActiveStorage::PurgeJob'", [], |r| r.get::<_, i64>(0))?,
            conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action LIKE 'soundboard_sound.%'", [], |r| r.get::<_, i64>(0))?,
        ))
    }).await.unwrap();
    assert_eq!(attachments, 0);
    assert!(purges > 0);
    assert_eq!(audits, 3);
    for method in [Method::PATCH, Method::DELETE] {
        let missing = write(
            &mut admin,
            method,
            &path,
            json!({"name":"Missing","emoji":null,"volume":1.0}),
        )
        .await;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn soundboard_uses_the_existing_direct_upload_api() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let reply = write(
        &mut admin,
        Method::POST,
        "/api/v1/uploads",
        json!({
            "filename":"sound.wav", "byteSize":WAV.len(),
            "checksum":campfire_storage::key::checksum(WAV), "contentType":"audio/wav"
        }),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let upload: api::DirectUpload = parse(&reply);
    let reply = admin
        .send(
            Req::new(Method::PUT, &upload.upload_url)
                .header("content-type", "audio/wav")
                .body(WAV),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let created = write(
        &mut admin,
        Method::POST,
        ADMIN,
        json!({"name":"Uploaded","signedId":upload.signed_id}),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let catalog: api::SoundboardCatalog = parse(&created);
    assert_eq!(catalog.sounds.len(), 1);
    let fetched = admin.get(&catalog.sounds[0].audio_url).await;
    assert_eq!(fetched.status, StatusCode::OK);
    assert_eq!(fetched.body, WAV);
}

#[tokio::test]
async fn soundboard_accepts_mp3_with_embedded_cover_art() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("artwork.mp3");
    let result = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-threads",
            "1",
            "-filter_threads",
            "1",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=mono",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=16x16:r=10",
            "-map",
            "0:a",
            "-map",
            "1:v",
            "-t",
            "0.1",
            "-c:a",
            "libmp3lame",
            "-c:v",
            "mjpeg",
            "-disposition:v",
            "attached_pic",
            "-frames:v",
            "1",
            "-id3v2_version",
            "3",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let signed_id = upload(
        &a,
        &std::fs::read(&path).unwrap(),
        "artwork.mp3",
        "audio/mpeg",
    )
    .await;
    let reply = write(
        &mut admin,
        Method::POST,
        ADMIN,
        json!({"name":"Cover art","signedId":signed_id}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    assert_eq!(parse::<api::SoundboardCatalog>(&reply).sounds.len(), 1);
}

#[tokio::test]
async fn soundboard_management_is_admin_only_and_catalog_requires_sign_in() {
    let Some(a) = app().await else { return };
    let mut member = a.sign_in(KEVIN).await;
    for (method, path) in [
        (Method::POST, ADMIN.to_string()),
        (Method::PATCH, format!("{ADMIN}/999")),
        (Method::DELETE, format!("{ADMIN}/999")),
    ] {
        let reply = write(&mut member, method, &path, json!({})).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
        assert_eq!(error(&reply)["_tag"], "Forbidden");
    }
    let reply = a.anonymous().send(get(CATALOG)).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(error(&reply)["_tag"], "Unauthorized");
    assert_eq!(
        parse::<api::SoundboardCatalog>(&member.send(get(CATALOG)).await).sounds,
        []
    );
}

#[tokio::test]
async fn soundboard_rejects_invalid_uploads_names_and_volume_without_catalog_writes() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let signed_id = upload(&a, WAV, "sound.wav", "audio/wav").await;
    for (input, field) in [
        (json!({"name":"","signedId":signed_id}), "name"),
        (json!({"name":"x".repeat(33),"signedId":signed_id}), "name"),
        (
            json!({"name":"Invalid","emoji":"plain text","signedId":signed_id}),
            "emoji",
        ),
        (
            json!({"name":"Invalid","volume":-0.1,"signedId":signed_id}),
            "volume",
        ),
        (
            json!({"name":"Invalid","volume":1.1,"signedId":signed_id}),
            "volume",
        ),
        (json!({"name":"Invalid","signedId":"forged"}), "audio"),
    ] {
        let reply = write(&mut admin, Method::POST, ADMIN, input).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert!(error(&reply)["fields"][field].is_array());
    }
    let png = include_bytes!("../../../../fixtures/files/workspace_icons/square_64.png");
    let mut too_big = WAV.to_vec();
    too_big.resize(512 * 1024 + 1, 0);
    for (bytes, filename, content_type) in [
        (png.as_slice(), "image.png", "image/png"),
        (b"not audio".as_slice(), "file.flac", "audio/flac"),
        (too_big.as_slice(), "large.wav", "audio/wav"),
        (b"".as_slice(), "empty.wav", "audio/wav"),
        (b"not audio".as_slice(), "spoofed.wav", "audio/wav"),
    ] {
        let signed_id = upload(&a, bytes, filename, content_type).await;
        let reply = write(
            &mut admin,
            Method::POST,
            ADMIN,
            json!({"name":"Invalid","signedId":signed_id}),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert!(error(&reply)["fields"]["audio"].is_array());
    }
    assert_eq!(
        parse::<api::SoundboardCatalog>(&admin.send(get(CATALOG)).await).sounds,
        []
    );
}

#[tokio::test]
async fn soundboard_accepts_all_audio_formats_and_rejects_webm_with_video() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let dir = tempfile::tempdir().unwrap();
    for (index, format, codec, content_type) in [
        (0, "mp3", "libmp3lame", "audio/mpeg"),
        (1, "ogg", "libopus", "audio/ogg"),
        (2, "webm", "libopus", "audio/webm"),
    ] {
        let path = dir.path().join(format!("audio.{format}"));
        let result = std::process::Command::new("ffmpeg")
            .args([
                "-threads",
                "1",
                "-filter_threads",
                "1",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=48000:cl=mono",
                "-t",
                "0.1",
                "-c:a",
                codec,
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = std::fs::read(&path).unwrap();
        let signed_id = upload(&a, &bytes, &format!("audio.{format}"), content_type).await;
        let reply = write(
            &mut admin,
            Method::POST,
            ADMIN,
            json!({"name":format!("Format {index}"),"signedId":signed_id}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    }
    let mut maximum = WAV.to_vec();
    maximum.resize(512 * 1024, 0);
    let signed_id = upload(&a, &maximum, "maximum.wav", "audio/wav").await;
    let reply = write(
        &mut admin,
        Method::POST,
        ADMIN,
        json!({"name":"Maximum WAV","signedId":signed_id}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let path = dir.path().join("video.webm");
    let result = std::process::Command::new("ffmpeg")
        .args([
            "-threads",
            "1",
            "-filter_threads",
            "1",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=16x16:r=10",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=mono",
            "-t",
            "0.1",
            "-c:v",
            "libvpx",
            "-c:a",
            "libopus",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(path).unwrap();
    let signed_id = upload(&a, &bytes, "video.webm", "audio/webm").await;
    let (_, spoofed_id) = legacy_upload(&mut admin, &bytes, "audio/mpeg", json!(true)).await;
    for signed_id in [signed_id, spoofed_id] {
        let reply = write(
            &mut admin,
            Method::POST,
            ADMIN,
            json!({"name":"Video","signedId":signed_id}),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
    }
    assert_eq!(
        parse::<api::SoundboardCatalog>(&admin.send(get(CATALOG)).await)
            .sounds
            .len(),
        4
    );
}

#[tokio::test]
async fn soundboard_uniqueness_cap_and_updates_at_capacity() {
    let Some(a) = app().await else { return };
    let mut admin = a.sign_in(DAVID).await;
    let signed_id = upload(&a, WAV, "sound.wav", "audio/wav").await;
    let mut first_id = 0;
    for index in 0..48 {
        let reply = write(
            &mut admin,
            Method::POST,
            ADMIN,
            json!({"name":format!("Sound {index:02}"),"signedId":signed_id}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        if index == 0 {
            first_id = parse::<api::SoundboardCatalog>(&reply).sounds[0].id;
            let duplicate = write(
                &mut admin,
                Method::POST,
                ADMIN,
                json!({"name":"sound 00","signedId":signed_id}),
            )
            .await;
            assert_eq!(duplicate.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                error(&duplicate)["fields"]["name"],
                json!(["has already been taken"])
            );
        }
    }
    let reply = write(
        &mut admin,
        Method::POST,
        ADMIN,
        json!({"name":"Overflow","signedId":signed_id}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error(&reply)["fields"]["base"],
        json!(["soundboard capacity reached (limit: 48 sounds)"])
    );
    let path = format!("{ADMIN}/{first_id}");
    let duplicate = write(
        &mut admin,
        Method::PATCH,
        &path,
        json!({"name":"sOUND 01","emoji":null,"volume":1.0}),
    )
    .await;
    assert_eq!(duplicate.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error(&duplicate)["fields"]["name"],
        json!(["has already been taken"])
    );
    let changed = write(
        &mut admin,
        Method::PATCH,
        &path,
        json!({"name":"Changed","emoji":"🎵","volume":0.0}),
    )
    .await;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.text());
    let deleted = write(&mut admin, Method::DELETE, &path, Value::Null).await;
    assert_eq!(deleted.status, StatusCode::OK);
    let restored = write(
        &mut admin,
        Method::POST,
        ADMIN,
        json!({"name":"Replacement","signedId":signed_id}),
    )
    .await;
    assert_eq!(restored.status, StatusCode::CREATED, "{}", restored.text());
    assert_eq!(parse::<api::SoundboardCatalog>(&restored).sounds.len(), 48);
}
