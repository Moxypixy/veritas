use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine, engine::general_purpose::STANDARD};

use crate::ApiError;

const NONCE_LEN: usize = 12;

pub struct DataKey([u8; 32]);

impl DataKey {
    pub fn from_base64(value: &str) -> Result<Self, ApiError> {
        let bytes = STANDARD
            .decode(value)
            .map_err(|_| ApiError::unavailable("VERITAS_DATA_KEY must be valid base64"))?;
        let key = bytes.try_into().map_err(|_| {
            ApiError::unavailable("VERITAS_DATA_KEY must decode to exactly 32 bytes")
        })?;
        Ok(Self(key))
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), ApiError> {
        let cipher = Aes256Gcm::new_from_slice(&self.0)
            .map_err(|_| ApiError::unavailable("could not initialize answer encryption"))?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| ApiError::unavailable("could not encrypt answer"))?;
        Ok((ciphertext, nonce.to_vec()))
    }

    pub fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, ApiError> {
        if nonce.len() != NONCE_LEN {
            return Err(ApiError::unavailable("stored answer has an invalid nonce"));
        }
        let cipher = Aes256Gcm::new_from_slice(&self.0)
            .map_err(|_| ApiError::unavailable("could not initialize answer encryption"))?;
        cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| ApiError::unavailable("could not decrypt stored answer"))
    }
}
