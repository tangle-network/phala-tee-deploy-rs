use crate::error::Error;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use curve25519_dalek::scalar::Scalar;
use rand::{rngs::OsRng, Rng, RngCore};

/// Handles encryption operations for TEE deployments
pub struct Encryptor {
    private_key: Scalar,
    public_key: [u8; 32],
}

impl Encryptor {
    /// Create a new Encryptor with a randomly generated key pair
    pub fn new() -> Self {
        let mut rng = OsRng;
        let private_key = Scalar::from_bytes_mod_order(rng.gen());
        let public_key = (curve25519_dalek::constants::ED25519_BASEPOINT_POINT * private_key)
            .compress()
            .to_bytes();

        Self {
            private_key,
            public_key,
        }
    }

    /// Encrypt environment variables using x25519 key exchange and AES-GCM
    pub fn encrypt_env_vars(
        &self,
        env_vars: &[(String, String)],
        remote_pubkey_hex: &str,
    ) -> Result<String, Error> {
        // Decode remote public key
        let remote_pubkey = hex::decode(remote_pubkey_hex.trim_start_matches("0x"))
            .map_err(|e| Error::InvalidKey(e.to_string()))?;

        if remote_pubkey.len() != 32 {
            return Err(Error::InvalidKey("Invalid remote public key length".into()));
        }

        // Convert environment variables to JSON
        let env_json = serde_json::json!({
            "env": env_vars.iter().map(|(k, v)| {
                serde_json::json!({
                    "key": k,
                    "value": v,
                })
            }).collect::<Vec<_>>()
        });
        let env_data =
            serde_json::to_string(&env_json).map_err(|e| Error::Encryption(e.to_string()))?;

        // Generate random IV
        let mut iv = [0u8; 12];
        OsRng.fill_bytes(&mut iv);
        let nonce = Nonce::from_slice(&iv);

        // Perform key exchange and derive shared secret
        let remote_point =
            curve25519_dalek::edwards::CompressedEdwardsY(remote_pubkey.try_into().unwrap())
                .decompress()
                .ok_or_else(|| Error::InvalidKey("Invalid remote public key point".into()))?;

        let shared_secret = (remote_point * self.private_key).compress().to_bytes();

        // Create AES cipher
        let key = Key::<Aes256Gcm>::from_slice(&shared_secret);
        let cipher = Aes256Gcm::new(key);

        // Encrypt data
        let encrypted = cipher
            .encrypt(nonce, env_data.as_bytes())
            .map_err(|e| Error::Encryption(e.to_string()))?;

        // Combine components: public key + IV + encrypted data
        let mut result = Vec::with_capacity(32 + 12 + encrypted.len());
        result.extend_from_slice(&self.public_key);
        result.extend_from_slice(&iv);
        result.extend_from_slice(&encrypted);

        Ok(hex::encode(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_flow() {
        let encryptor = Encryptor::new();
        let remote_pubkey = "0x".to_string() + &hex::encode([1u8; 32]);

        let env_vars = vec![
            ("KEY1".to_string(), "value1".to_string()),
            ("KEY2".to_string(), "value2".to_string()),
        ];

        let result = encryptor.encrypt_env_vars(&env_vars, &remote_pubkey);
        assert!(result.is_ok());

        let encrypted = result.unwrap();
        assert!(encrypted.len() > 32 + 12); // public key + IV + some encrypted data
    }
}
