//! `reference/app/models/sound.rb`

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundImage {
    /// `"sounds/#{name}"`
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
}

impl SoundImage {
    pub fn asset_path(&self) -> String {
        format!("sounds/{}", self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sound {
    pub name: &'static str,
    pub text: Option<&'static str>,
    pub image: Option<SoundImage>,
}

const fn text(name: &'static str, text: &'static str) -> Sound {
    Sound {
        name,
        text: Some(text),
        image: None,
    }
}

const fn image(name: &'static str, file: &'static str, width: u32, height: u32) -> Sound {
    Sound {
        name,
        text: None,
        image: Some(SoundImage {
            name: file,
            width,
            height,
        }),
    }
}

impl Sound {
    /// `"#{name}.mp3"`
    pub fn asset_path(&self) -> String {
        format!("{}.mp3", self.name)
    }

    pub fn find_by_name(name: &str) -> Option<&'static Sound> {
        BUILTIN.iter().find(|s| s.name == name)
    }

    /// `Sound.names`: sorted.
    pub fn names() -> Vec<&'static str> {
        let mut names: Vec<_> = BUILTIN.iter().map(|s| s.name).collect();
        names.sort();
        names
    }
}

pub const BUILTIN: &[Sound] = &[
    image("56k", "56k.webp", 79, 33),
    text("bell", "🔔"),
    text("bezos", "😆💭"),
    text("bueller", "anyone?"),
    text("butts", "👐 🚬"),
    image("clowntown", "clowntown.webp", 210, 150),
    text("cottoneyejoe", "🎶🙉🎶 "),
    text("crickets", "hears crickets chirping"),
    image("curb", "curb.webp", 150, 101),
    text("dadgummit", "dad gummit!! 🎣"),
    image("dangerzone", "dangerzone.webp", 157, 32),
    text("danielsan", "🎆 🏆 🎆"),
    image("deeper", "top.webp", 188, 80),
    text("ballmer", "developers!"),
    image("donotwant", "donotwant.webp", 150, 150),
    image("drama", "drama.webp", 300, 16),
    text("flawless", "#flawless"),
    text("glados", "🤖💢"),
    text("gogogo", "Go, go, go!"),
    image("greatjob", "greatjob.webp", 79, 16),
    text("greyjoy", "😖🎺"),
    text("guarantee", "guarantees it 👌"),
    text("heygirl", "✨💁✨"),
    text("honk", "HONK"),
    text("horn", "🐶 ✂️ 🐱"),
    text("horror", "💀 💀 💀 💀 💀 💀 💀"),
    text(
        "inconceivable",
        "doesn't think it means what you think it means…",
    ),
    text("letitgo", "❄️👩❄️⛄️❄️"),
    text("live", "is DOING IT LIVE"),
    image("loggins", "loggins.webp", 200, 151),
    text("makeitso", "make it so 👉"),
    text("noooo", "👸💀😒"),
    image("nyan", "nyan.webp", 36, 15),
    text("ohmy", "raises an eyebrow 😏"),
    text("ohyeah", "isn't playing by the rules"),
    image("pushit", "pushit.webp", 104, 15),
    text("rimshot", "plays a rimshot"),
    text("rollout", "is rolling out 🚗"),
    image("rumble", "rumble.webp", 220, 150),
    text("sax", "🌇🎷🎶"),
    text("secret", "found a secret area 🔑"),
    text("sexyback", "🔞"),
    text("story", "and now you know…"),
    text("tada", "plays a fanfare 🎏"),
    text("tmyk", "✨ ⭐️ The More You Know ✨ ⭐️"),
    text("totes", "😁👍"),
    text("trololo", "трололо"),
    text("trombone", "plays a sad trombone"),
    text("unix", "knows this 💻"),
    text("vuvuzela", "======<() ~ ♪ ~♫"),
    image("what", "what.webp", 100, 131),
    text("whoomp", "👏‼️😎"),
    text("wups", "wups!"),
    image("yay", "yay.webp", 103, 50),
    image("yeah", "yeah.webp", 104, 15),
    text("yodel", "📣🗻🙉"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;

    /// `BUILTIN` is the list from the reference's `app/models/sound.rb`, checked against it
    /// until Rails was removed; it is the source of truth now. The hash pins every name, text
    /// and image (one `name\ttext:TEXT` or `name\timage:FILE:WIDTH:HEIGHT` line per sound, in
    /// order), computed from `sound.rb` at removal, so a rename or changed text fails here.
    #[test]
    fn builtin_sounds_are_the_reference_list() {
        let names: std::collections::BTreeSet<_> = BUILTIN.iter().map(|s| s.name).collect();
        assert_eq!((BUILTIN.len(), names.len()), (56, 56));
        let listing: String = BUILTIN
            .iter()
            .map(|sound| match (sound.text, sound.image) {
                (Some(text), None) => format!("{}\ttext:{text}\n", sound.name),
                (None, Some(image)) => format!("{}\timage:{}:{}:{}\n", sound.name, image.name, image.width, image.height),
                other => panic!("{}: a sound has text or an image: {other:?}", sound.name),
            })
            .collect();
        assert_eq!(
            hex::encode(sha2::Sha256::digest(listing)),
            "afa51b9412ac810146054e2ab501e4b7fac5087495fccbc7070d49cd7b3b8db5",
            "the builtin sounds changed"
        );
        assert_eq!(
            Sound::find_by_name("deeper")
                .unwrap()
                .image
                .unwrap()
                .asset_path(),
            "sounds/top.webp"
        );
    }

    /// Every builtin sound plays `NAME.mp3` and shows its image from the port's assets.
    #[test]
    fn builtin_sound_assets_exist() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/app/assets");
        for sound in BUILTIN {
            let mp3 = assets.join(format!("sounds/{}.mp3", sound.name));
            assert!(mp3.is_file(), "{} is missing", mp3.display());
            if let Some(image) = sound.image {
                let file = assets.join("images").join(image.asset_path());
                assert!(file.is_file(), "{} is missing", file.display());
            }
        }
    }
}
