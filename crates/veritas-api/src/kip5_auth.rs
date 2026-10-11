//! KIP-5 wallet login, as signed by Kaspire's `signMessage`.
//!
//! A login is accepted only when:
//! 1. `wallet` is a Kaspa testnet (`kaspatest:`) Schnorr public-key address;
//! 2. that address encodes exactly `public_key` (32-byte x-only, hex); and
//! 3. `signature` (64-byte hex) is a KIP-5 signature by that key over the
//!    server-issued challenge text.

use async_trait::async_trait;
use kaspa_addresses::{Address, Prefix, Version};

use crate::{ApiError, AuthVerifier};

pub struct Kip5AuthVerifier {
    prefix: Prefix,
}

impl Kip5AuthVerifier {
    /// Accepts only Testnet-10 addresses (`kaspatest:`), matching v1 scope.
    pub fn testnet() -> Self {
        Self {
            prefix: Prefix::Testnet,
        }
    }
}

#[async_trait]
impl AuthVerifier for Kip5AuthVerifier {
    async fn verify(
        &self,
        wallet: &str,
        challenge: &str,
        signature: &str,
        public_key: &str,
    ) -> Result<bool, ApiError> {
        let Ok(address) = Address::try_from(wallet.trim()) else {
            return Ok(false);
        };
        if address.prefix != self.prefix || address.version != Version::PubKey {
            return Ok(false);
        }
        let (Ok(public_key), Ok(signature)) = (
            veritas::hex::decode_array::<32>(public_key),
            veritas::hex::decode_array::<64>(signature.trim().trim_start_matches("0x")),
        ) else {
            return Ok(false);
        };
        if address.payload.as_slice() != public_key {
            return Ok(false);
        }
        Ok(veritas::kip5::verify(
            challenge.as_bytes(),
            &signature,
            &public_key,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::{Keypair, Secp256k1, SecretKey};

    fn keypair(byte: u8) -> Keypair {
        Keypair::from_secret_key(
            &Secp256k1::new(),
            &SecretKey::from_slice(&[byte; 32]).unwrap(),
        )
    }

    fn login(byte: u8, prefix: Prefix) -> (String, String, [u8; 32]) {
        let keypair = keypair(byte);
        let public_key = keypair.x_only_public_key().0.serialize();
        let address = Address::new(prefix, Version::PubKey, &public_key).to_string();
        let challenge = format!("Veritas login\nwallet={address}\nnonce=1");
        (address, challenge, public_key)
    }

    async fn check(wallet: &str, challenge: &str, signature: &[u8; 64], key: &[u8; 32]) -> bool {
        Kip5AuthVerifier::testnet()
            .verify(
                wallet,
                challenge,
                &veritas::hex::encode(signature),
                &veritas::hex::encode(key),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn accepts_a_kip5_signature_from_the_address_owner() {
        let (address, challenge, key) = login(3, Prefix::Testnet);
        let signature = veritas::kip5::sign(challenge.as_bytes(), &keypair(3));
        assert!(check(&address, &challenge, &signature, &key).await);
    }

    #[tokio::test]
    async fn rejects_another_challenge_key_or_network() {
        let (address, challenge, key) = login(3, Prefix::Testnet);
        let signature = veritas::kip5::sign(challenge.as_bytes(), &keypair(3));
        assert!(!check(&address, "Veritas login\nnonce=2", &signature, &key).await);

        // A valid signature by key 4, presented with key 4, for key 3's address.
        let other = keypair(4);
        let other_key = other.x_only_public_key().0.serialize();
        let other_sig = veritas::kip5::sign(challenge.as_bytes(), &other);
        assert!(!check(&address, &challenge, &other_sig, &other_key).await);

        let (mainnet, mainnet_challenge, mainnet_key) = login(3, Prefix::Mainnet);
        let mainnet_sig = veritas::kip5::sign(mainnet_challenge.as_bytes(), &keypair(3));
        assert!(!check(&mainnet, &mainnet_challenge, &mainnet_sig, &mainnet_key).await);
    }

    #[tokio::test]
    async fn malformed_input_is_a_plain_rejection() {
        let verifier = Kip5AuthVerifier::testnet();
        assert!(
            !verifier
                .verify("not-an-address", "c", "00", "00")
                .await
                .unwrap()
        );
    }
}
