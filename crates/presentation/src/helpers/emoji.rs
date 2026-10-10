// `EmojiHelper` (`reference/app/helpers/emoji_helper.rb`).

/// `EmojiHelper::REACTIONS`: the quick reactions, as character and title, in order.
pub const REACTIONS: [(&str, &str); 8] = [
    ("👍", "Thumbs up"),
    ("👏", "Clapping"),
    ("👋", "Waving hand"),
    ("💪", "Muscle"),
    ("❤️", "Red heart"),
    ("😂", "Face with tears of joy"),
    ("🎉", "Party popper"),
    ("🔥", "Fire"),
];

/// The emoji picker's category tabs (`messages/_emoji_picker.html.erb`): id, name and glyph.
pub const EMOJI_PICKER_TABS: [(&str, &str, &str); 11] = [
    ("recent", "Recent", "🕒"),
    ("smileys", "Smileys", "😀"),
    ("people", "People", "👋"),
    ("nature", "Nature", "🐻"),
    ("food", "Food", "🍔"),
    ("activities", "Activities", "⚽"),
    ("travel", "Travel", "✈️"),
    ("objects", "Objects", "💡"),
    ("symbols", "Symbols", "🔣"),
    ("flags", "Flags", "🏳️"),
    ("custom", "Custom", "🧩"),
];
