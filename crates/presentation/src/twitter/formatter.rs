pub fn clamp(text: &str) -> bool { text.chars().count() > 480 || text.bytes().filter(|b| *b == b'\n').count() >= 12 }
