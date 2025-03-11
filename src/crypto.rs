use crate::error::Error;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use x25519_dalek::{EphemeralSecret, PublicKey};

/// Cryptographic utilities for secure data transmission.
///
/// This struct provides methods for encrypting sensitive data, particularly
/// environment variables, using industry-standard cryptographic algorithms.
/// It implements the same encryption scheme as the TypeScript client to ensure
/// compatibility with the Phala TEE Cloud platform.
pub struct Encryptor;

impl Encryptor {
    /// Encrypts environment variables using X25519 key exchange and AES-GCM.
    ///
    /// This method implements a hybrid encryption scheme:
    /// 1. X25519 for key exchange (establishes a shared secret)
    /// 2. AES-GCM for authenticated encryption of the actual data
    ///
    /// The process is compatible with the TypeScript implementation used by
    /// the Phala Cloud API.
    ///
    /// # Parameters
    ///
    /// * `env_vars` - A slice of key-value pairs representing environment variables to encrypt
    /// * `remote_pubkey_hex` - The remote public key as a hex string (with or without '0x' prefix)
    ///
    /// # Returns
    ///
    /// A hex-encoded string containing the ephemeral public key, IV, and encrypted data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The public key is not valid hex or has incorrect length
    /// * JSON serialization fails
    /// * Encryption fails
    pub fn encrypt_env_vars(
        env_vars: &[(String, String)],
        remote_pubkey_hex: &str,
    ) -> Result<String, Error> {
        println!("Encrypting environment variables");

        // Decode remote public key (remove 0x prefix if present)
        let clean_pubkey = remote_pubkey_hex.trim_start_matches("0x");
        let remote_pubkey_bytes = hex::decode(clean_pubkey)
            .map_err(|e| Error::InvalidKey(format!("Invalid hex encoding: {}", e)))?;

        if remote_pubkey_bytes.len() != 32 {
            return Err(Error::InvalidKey(format!(
                "Invalid public key length: expected 32 bytes, got {}",
                remote_pubkey_bytes.len()
            )));
        }

        // Convert to PublicKey
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&remote_pubkey_bytes);
        let remote_pubkey = PublicKey::from(key_bytes);

        // Generate the shared secret via Diffie-Hellman
        // Create a new EphemeralSecret for each encryption operation
        let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
        let public_key = PublicKey::from(&ephemeral_secret);
        let shared_secret = ephemeral_secret.diffie_hellman(&remote_pubkey);

        // Convert environment variables to the expected JSON format
        let env_json = serde_json::json!({
            "env": env_vars.iter().map(|(k, v)| {
                serde_json::json!({
                    "key": k,
                    "value": v,
                })
            }).collect::<Vec<_>>()
        });

        let env_data = serde_json::to_string(&env_json)
            .map_err(|e| Error::Encryption(format!("JSON serialization error: {}", e)))?;

        // Generate a random 12-byte nonce (IV)
        let mut iv = [0u8; 12];
        OsRng.fill_bytes(&mut iv);
        let nonce = Nonce::from_slice(&iv);

        // Create the AES-GCM cipher using the shared secret as the key
        let key = Key::<Aes256Gcm>::from_slice(shared_secret.as_bytes());
        let cipher = Aes256Gcm::new(key);

        // Encrypt the data
        let encrypted = cipher
            .encrypt(nonce, env_data.as_bytes())
            .map_err(|e| Error::Encryption(format!("AES encryption error: {}", e)))?;

        // Combine components as in TypeScript: public key + IV + encrypted data
        let mut result = Vec::with_capacity(32 + 12 + encrypted.len());
        result.extend_from_slice(public_key.as_bytes());
        result.extend_from_slice(&iv);
        result.extend_from_slice(&encrypted);

        // Return hex-encoded result
        Ok(hex::encode(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_flow() {
        let remote_pubkey = "0x".to_string() + &hex::encode([1u8; 32]);

        let env_vars = vec![
            ("KEY1".to_string(), "value1".to_string()),
            ("KEY2".to_string(), "value2".to_string()),
        ];

        let result = Encryptor::encrypt_env_vars(&env_vars, &remote_pubkey);
        assert!(result.is_ok());

        let encrypted = result.unwrap();
        assert!(encrypted.len() > 32 + 12); // public key + IV + some encrypted data
    }
}
