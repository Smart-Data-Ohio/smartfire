//! `has_secure_password` with `ActiveModel::SecurePassword::BCryptPassword`: bcrypt-ruby's
//! `$2a$` digests at `BCrypt::Engine.cost` (12). Like bcrypt-ruby, only the first 72 bytes of a
//! password count.

/// `BCrypt::Engine.cost` in production.
pub const COST: u32 = 12;
/// `BCrypt::Engine::MIN_COST`, what Rails uses when `ActiveModel::SecurePassword.min_cost` is set (tests).
pub const MIN_COST: u32 = 4;

/// `BCrypt::Password.create(password, cost: COST)`.
pub fn digest(password: &str) -> String {
    digest_with_cost(password, COST)
}

pub fn digest_with_cost(password: &str, cost: u32) -> String {
    bcrypt::hash_with_result(password, cost).expect("cost is in range").format_for_version(bcrypt::Version::TwoA)
}

/// `BCrypt::Password.new(digest).is_password?(password)`; `false` for a malformed digest.
pub fn verify(password: &str, digest: &str) -> bool {
    bcrypt::verify(password, digest).unwrap_or(false)
}
