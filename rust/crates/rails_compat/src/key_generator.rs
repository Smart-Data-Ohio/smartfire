use std::collections::HashMap;
use std::sync::Mutex;

/// `Rails.application.key_generator`: an `ActiveSupport::CachingKeyGenerator` over PBKDF2-HMAC.
/// Rails builds it with 1000 iterations (railties `Rails::Application#key_generator`), and
/// `load_defaults` 7.0+ sets its digest to SHA256 (`key_generator_hash_digest_class`). The
/// default key length is 64 bytes; encrypted cookies ask for 32.
pub struct KeyGenerator {
    secret: String,
    cache: Mutex<HashMap<(String, usize), Vec<u8>>>,
}

pub const ITERATIONS: u32 = 1000;
pub const DEFAULT_KEY_LENGTH: usize = 64;

impl KeyGenerator {
    pub fn new(secret_key_base: &str) -> Self {
        Self { secret: secret_key_base.to_string(), cache: Mutex::new(HashMap::new()) }
    }

    pub fn generate_key(&self, salt: &str, length: usize) -> Vec<u8> {
        let mut cache = self.cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cache
            .entry((salt.to_string(), length))
            .or_insert_with(|| {
                let mut key = vec![0u8; length];
                pbkdf2::pbkdf2_hmac::<sha2::Sha256>(self.secret.as_bytes(), salt.as_bytes(), ITERATIONS, &mut key);
                key
            })
            .clone()
    }
}
