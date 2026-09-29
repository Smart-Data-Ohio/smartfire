use std::collections::HashMap;
use std::sync::Mutex;

/// `ActiveSupport::KeyGenerator` (with `CachingKeyGenerator`'s memoization): PBKDF2-HMAC over a
/// secret.
///
/// [`KeyGenerator::new`] is Smartfire's `Rails.application.key_generator`, which is
/// PBKDF2-HMAC-**SHA1** with 1000 iterations, not the SHA256 that `load_defaults` 7.0+ asks for.
/// `config/initializers/active_record_encryption.rb` calls `Rails.application.key_generator`
/// while initializers run, which memoizes it (railties `Rails::Application#key_generator`) before
/// the `active_support.set_key_generator_hash_digest_class` hook switches
/// `ActiveSupport::KeyGenerator.hash_digest_class` to SHA256 in `after_initialize`. Everything the
/// app derives from `secret_key_base` (cookies, the session, signed ids, SGIDs, Turbo stream
/// names, `message_verifier(name)`, Active Storage, the Active Record encryption keys) uses it.
/// `vectors/rails_compat.json`'s `key_generator` section pins this.
///
/// Active Record encryption derives its cipher key with a separate SHA256 generator at the
/// default 2**16 iterations: see [`crate::ar_encryption`].
pub struct KeyGenerator {
    secret: Vec<u8>,
    digest: HashDigest,
    iterations: u32,
    cache: Mutex<HashMap<(Vec<u8>, usize), Vec<u8>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashDigest {
    Sha1,
    Sha256,
}

/// `Rails::Application#key_generator`'s iteration count.
pub const ITERATIONS: u32 = 1000;
/// `ActiveSupport::KeyGenerator`'s own default, used when no `iterations:` is given.
pub const DEFAULT_ITERATIONS: u32 = 1 << 16;
pub const DEFAULT_KEY_LENGTH: usize = 64;

impl KeyGenerator {
    /// `Rails.application.key_generator` as Smartfire builds it: SHA1, 1000 iterations.
    pub fn new(secret_key_base: &str) -> Self {
        Self::with_options(secret_key_base.as_bytes(), HashDigest::Sha1, ITERATIONS)
    }

    /// `ActiveSupport::KeyGenerator.new(secret, hash_digest_class:, iterations:)`.
    pub fn with_options(secret: &[u8], digest: HashDigest, iterations: u32) -> Self {
        Self { secret: secret.to_vec(), digest, iterations, cache: Mutex::new(HashMap::new()) }
    }

    pub fn generate_key(&self, salt: &str, length: usize) -> Vec<u8> {
        self.generate_key_from_bytes(salt.as_bytes(), length)
    }

    /// For binary salts (Active Record encryption's `key_derivation_salt`).
    pub fn generate_key_from_bytes(&self, salt: &[u8], length: usize) -> Vec<u8> {
        let mut cache = self.cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.entry((salt.to_vec(), length)).or_insert_with(|| self.derive(salt, length)).clone()
    }

    fn derive(&self, salt: &[u8], length: usize) -> Vec<u8> {
        let mut key = vec![0u8; length];
        match self.digest {
            HashDigest::Sha1 => pbkdf2::pbkdf2_hmac::<sha1::Sha1>(&self.secret, salt, self.iterations, &mut key),
            HashDigest::Sha256 => pbkdf2::pbkdf2_hmac::<sha2::Sha256>(&self.secret, salt, self.iterations, &mut key),
        }
        key
    }
}
