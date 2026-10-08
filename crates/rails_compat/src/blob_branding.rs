//! Server-owned provenance for workspace images, bound to each Active Storage blob key.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::Secrets;

#[derive(Clone, Copy)]
pub struct Marker([u8; 32]);

impl std::fmt::Debug for Marker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Marker")
    }
}

impl Marker {
    pub fn new(secrets: &Secrets) -> Self {
        let key = secrets
            .key_generator
            .generate_key("ActiveStorage/branding", 32);
        Self(key.try_into().expect("32-byte branding key"))
    }

    fn mac(&self, blob_key: &str) -> Hmac<Sha256> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.0).expect("HMAC takes any key length");
        mac.update(b"branding:");
        mac.update(blob_key.as_bytes());
        mac
    }

    pub fn sign(&self, blob_key: &str) -> String {
        hex::encode(self.mac(blob_key).finalize().into_bytes())
    }

    pub fn verifies(&self, blob_key: &str, mark: &str) -> bool {
        if mark.len() != 64 {
            return false;
        }
        let Ok(signature) = hex::decode(mark) else {
            return false;
        };
        self.mac(blob_key).verify_slice(&signature).is_ok()
    }
}
