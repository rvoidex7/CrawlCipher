pub mod entropy;
pub mod profile;

use stellar_strkey::ed25519::PrivateKey;

pub fn validate_secret_key(secret: &str) -> Option<String> {
    match PrivateKey::from_string(secret) {
        Ok(_key) => {
            // Derive public key (G...) from secret key (S...)
            // stellar-strkey doesn't do derivation, it just formats.
            // We need ed25519-dalek for derivation.
            // For MVP simplicity: Just return true if format is valid.
            // But let's try to be helpful and return the public key if possible.
            // Since we can't easily derive without the seed bytes, we just validate format.
            Some("Public Key Derivation Not Implemented in MVP".to_string())
        },
        Err(_) => None,
    }
}
pub mod session;
