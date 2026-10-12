use super::{TestDb, id};
use crate::models::soundboard::{
    AudioFacts, CAPACITY, MAX_AUDIO_BYTES, SoundboardSound, SoundboardValues,
};
use crate::{Clock, Error, Timestamp};

fn values(name: &str) -> SoundboardValues {
    SoundboardValues {
        name: name.into(),
        emoji: None,
        volume: 1.0,
    }
}

fn audio() -> AudioFacts {
    AudioFacts {
        content_type: "audio/wav".into(),
        byte_size: 100,
    }
}

fn create(t: &TestDb, name: &str) -> SoundboardSound {
    let values = values(name);
    t.write(move |tx| SoundboardSound::create(tx, id("david"), values, &audio()))
}

#[test]
fn soundboard_validates_names_emoji_volume_and_audio() {
    let t = TestDb::new();
    for name in ["", " ", &"a".repeat(33), &"🎵".repeat(33)] {
        let input = values(name);
        assert!(matches!(
            t.try_write(move |tx| SoundboardSound::create(tx, id("david"), input, &audio())),
            Err(Error::RecordInvalid(_))
        ));
    }
    for volume in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        let input = SoundboardValues {
            volume,
            ..values("Invalid")
        };
        assert!(matches!(
            t.try_write(move |tx| SoundboardSound::create(tx, id("david"), input, &audio())),
            Err(Error::RecordInvalid(_))
        ));
    }
    let invalid = SoundboardValues {
        emoji: Some(":custom:".into()),
        ..values("Invalid")
    };
    assert!(matches!(
        t.try_write(move |tx| SoundboardSound::create(tx, id("david"), invalid, &audio())),
        Err(Error::RecordInvalid(_))
    ));
    for (content_type, byte_size) in [
        ("image/png", 100),
        ("audio/flac", 100),
        ("audio/wav", 0),
        ("audio/wav", -1),
        ("audio/wav", MAX_AUDIO_BYTES + 1),
    ] {
        let facts = AudioFacts {
            content_type: content_type.into(),
            byte_size,
        };
        assert!(matches!(
            t.try_write(move |tx| SoundboardSound::create(
                tx,
                id("david"),
                values("Invalid"),
                &facts
            )),
            Err(Error::RecordInvalid(_))
        ));
    }
    for (index, content_type) in ["audio/mpeg", "audio/ogg", "audio/wav", "audio/webm"]
        .into_iter()
        .enumerate()
    {
        let facts = AudioFacts {
            content_type: content_type.into(),
            byte_size: MAX_AUDIO_BYTES,
        };
        let input = SoundboardValues {
            emoji: Some("🎵".into()),
            volume: 0.0,
            ..values(&format!("Valid {index}"))
        };
        let sound = t.write(move |tx| SoundboardSound::create(tx, id("david"), input, &facts));
        assert_eq!(sound.emoji.as_deref(), Some("🎵"));
        assert_eq!(sound.volume, 0.0);
        assert_eq!(sound.uploader_id, id("david"));
    }
    for (index, emoji) in ["👍🏽", "👨‍👩‍👧‍👦", "❤", "🇺🇸", "1️⃣"].into_iter().enumerate()
    {
        let input = SoundboardValues {
            emoji: Some(emoji.into()),
            ..values(&format!("Emoji {index}"))
        };
        assert_eq!(
            t.write(move |tx| SoundboardSound::create(tx, id("david"), input, &audio()))
                .emoji
                .as_deref(),
            Some(emoji)
        );
    }
    let unicode_name = "é".repeat(32);
    assert_eq!(create(&t, &unicode_name).name, unicode_name);
}

#[test]
fn soundboard_names_are_unique_by_unicode_case_fold_on_create_and_update() {
    let t = TestDb::new();
    let first = create(&t, "  Straße  ");
    assert_eq!(first.name, "Straße");
    for name in ["STRASSE", "straße"] {
        let input = values(name);
        let Err(Error::RecordInvalid(errors)) =
            t.try_write(move |tx| SoundboardSound::create(tx, id("david"), input, &audio()))
        else {
            panic!("duplicate name accepted")
        };
        assert_eq!(errors.on("name"), ["has already been taken"]);
    }
    let second = create(&t, "Other");
    assert!(matches!(
        t.try_write(move |tx| second.update(tx, values("STRASSE"))),
        Err(Error::RecordInvalid(_))
    ));
    t.clock.travel_to(Timestamp::from_second(1_790_000_000));
    let changed = t.write(move |tx| {
        first.update(
            tx,
            SoundboardValues {
                emoji: Some("🎵".into()),
                volume: 0.5,
                ..values("STRASSE")
            },
        )
    });
    assert_eq!(changed.name, "STRASSE");
    assert_eq!(changed.emoji.as_deref(), Some("🎵"));
    assert_eq!(changed.volume, 0.5);
    assert_eq!(changed.updated_at, t.clock.now());
    assert_ne!(changed.created_at, changed.updated_at);
    let cleared = t.write(move |tx| {
        changed.update(
            tx,
            SoundboardValues {
                emoji: Some(String::new()),
                ..values("STRASSE")
            },
        )
    });
    assert_eq!(cleared.emoji, None);
}

#[test]
fn soundboard_cap_allows_48_sounds_updates_at_capacity_and_reuse_after_deletion() {
    let t = TestDb::new();
    for index in 0..CAPACITY {
        create(&t, &format!("Sound {index:02}"));
    }
    let Err(Error::RecordInvalid(errors)) =
        t.try_write(|tx| SoundboardSound::create(tx, id("david"), values("Overflow"), &audio()))
    else {
        panic!("capacity exceeded")
    };
    assert_eq!(
        errors.on("base"),
        ["soundboard capacity reached (limit: 48 sounds)"]
    );
    let sounds = t.read(SoundboardSound::list);
    assert_eq!(sounds.len(), 48);
    let first = sounds[0].clone();
    let edited = t.write(move |tx| first.update(tx, values("Changed")));
    assert_eq!(edited.name, "Changed");
    t.write(move |tx| edited.destroy(tx));
    create(&t, "Replacement");
    assert_eq!(t.read(SoundboardSound::list).len(), 48);
}

#[test]
fn soundboard_concurrent_creates_cannot_overfill_the_last_slot() {
    let t = TestDb::new();
    for index in 0..47 {
        create(&t, &format!("Sound {index}"));
    }
    let start = std::sync::Arc::new(std::sync::Barrier::new(2));
    let attempts: Vec<_> = (0..2)
        .map(|index| {
            let db = t.db.clone();
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                db.write_blocking(move |tx| {
                    SoundboardSound::create(
                        tx,
                        id("david"),
                        values(&format!("Last {index}")),
                        &audio(),
                    )
                })
            })
        })
        .collect();
    let results: Vec<_> = attempts
        .into_iter()
        .map(|attempt| attempt.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(Error::RecordInvalid(_))))
            .count(),
        1
    );
    assert_eq!(t.read(SoundboardSound::list).len(), 48);
}
