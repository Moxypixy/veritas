//! KIP-5 personal message signing, as used by Kaspa wallets such as Kaspire.
//!
//! `digest = BLAKE2b-256(message, key = "PersonalMessageSigningHash")`, then a
//! BIP-340 Schnorr signature over `digest`. The distinct key keeps a message
//! signature from ever being a valid transaction signature.

use blake2b_simd::Params;
use secp256k1::{Keypair, Message, SECP256K1, XOnlyPublicKey, schnorr::Signature};

const KIP5_KEY: &[u8] = b"PersonalMessageSigningHash";

/// The 32-byte digest a KIP-5 signature commits to.
pub fn message_digest(message: &[u8]) -> [u8; 32] {
    let hash = Params::new().hash_length(32).key(KIP5_KEY).hash(message);
    let mut out = [0; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

/// Signs `message` per KIP-5 with fresh auxiliary randomness when available.
pub fn sign(message: &[u8], keypair: &Keypair) -> [u8; 64] {
    let msg = Message::from_digest(message_digest(message));
    let mut aux = [0u8; 32];
    let signature = match getrandom::fill(&mut aux) {
        Ok(()) => SECP256K1.sign_schnorr_with_aux_rand(&msg, keypair, &aux),
        Err(_) => SECP256K1.sign_schnorr_no_aux_rand(&msg, keypair),
    };
    *signature.as_ref()
}

/// Verifies a 64-byte KIP-5 signature against a 32-byte x-only public key.
pub fn verify(message: &[u8], signature: &[u8; 64], public_key: &[u8; 32]) -> bool {
    let (Ok(key), Ok(signature)) = (
        XOnlyPublicKey::from_slice(public_key),
        Signature::from_slice(signature),
    ) else {
        return false;
    };
    signature
        .verify(&Message::from_digest(message_digest(message)), &key)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::decode_array;
    use secp256k1::{Secp256k1, SecretKey};

    /// KIP-5 test vectors (kaspanet/kips, kip-0005.md).
    const VECTORS: [(&str, &str, &str, &str, &str); 3] = [
        (
            "0000000000000000000000000000000000000000000000000000000000000003",
            "F9308A019258C31049344F85F89D5229B531C845836F99B08601F113BCE036F9",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "Hello Kaspa!",
            "40B9BB2BE0AE02607279EDA64015A8D86E3763279170340B8243F7CE5344D77AFF1191598BAF2FD26149CAC3B4B12C2C433261C00834DB6098CB172AA48EF522",
        ),
        (
            "B7E151628AED2A6ABF7158809CF4F3C762E7160F38B4DA56A784D9045190CFEF",
            "DFF1D77F2A671C5F36183726DB2341BE58FEAE1DA2DECED843240F7B502BA659",
            "0000000000000000000000000000000000000000000000000000000000000001",
            "Hello Kaspa!",
            "EB9E8A3C547EB91B6A7592644F328F0648BDD21ABA3CD44787D429D4D790AA8B962745691F3B472ED8D65F3B770ECB4F777BD17B1D309100919B53E0E206B4C6",
        ),
        (
            "B7E151628AED2A6ABF7158809CF4F3C762E7160F38B4DA56A784D9045190CFEF",
            "DFF1D77F2A671C5F36183726DB2341BE58FEAE1DA2DECED843240F7B502BA659",
            "0000000000000000000000000000000000000000000000000000000000000001",
            "こんにちは世界",
            "810653D5F80206DB519672362ADD6C98DAD378844E5BA4D89A22C9F0C7092E8CECBA734FFF7922B656B4BE3F4B1F098899C95CB5C1023DCE3519208AFAFB59BC",
        ),
    ];

    #[test]
    fn reproduces_the_kip5_vectors() {
        for (secret, public, aux, message, expected) in VECTORS {
            let keypair = Keypair::from_secret_key(
                &Secp256k1::new(),
                &SecretKey::from_slice(&decode_array::<32>(secret).unwrap()).unwrap(),
            );
            assert_eq!(
                keypair.x_only_public_key().0.serialize(),
                decode_array::<32>(public).unwrap()
            );
            let msg = Message::from_digest(message_digest(message.as_bytes()));
            let signature = SECP256K1.sign_schnorr_with_aux_rand(
                &msg,
                &keypair,
                &decode_array::<32>(aux).unwrap(),
            );
            let expected = decode_array::<64>(expected).unwrap();
            assert_eq!(*signature.as_ref(), expected, "{message}");
            assert!(verify(
                message.as_bytes(),
                &expected,
                &decode_array::<32>(public).unwrap()
            ));
        }
    }

    #[test]
    fn rejects_other_messages_and_keys() {
        let (_, public, _, message, signature) = VECTORS[0];
        let signature = decode_array::<64>(signature).unwrap();
        let public = decode_array::<32>(public).unwrap();
        assert!(!verify(b"Hello Kaspa?", &signature, &public));
        assert!(!verify(
            message.as_bytes(),
            &signature,
            &decode_array::<32>(VECTORS[1].1).unwrap()
        ));
        assert!(!verify(message.as_bytes(), &signature, &[0; 32]));
    }

    #[test]
    fn fresh_signatures_verify() {
        let keypair =
            Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[5; 32]).unwrap());
        let public = keypair.x_only_public_key().0.serialize();
        assert!(verify(b"veritas", &sign(b"veritas", &keypair), &public));
    }
}
