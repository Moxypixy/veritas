//! The price record: canonical encoding, leaf hash and attester signature.
//!
//! Canonical layout (all integers little-endian, `lp(x)` = u32 length + bytes):
//!
//! ```text
//! lp("veritas.price.v1") lp(item) amount_minor:u64 currency:[3]
//! qty_milli:u64 lp(unit_code) lp(place) observed_on:[10] source:u8
//! evidence:[32] salt:[16] attester:[32]
//! ```
//!
//! Only integers are hashed, so every device produces the same bytes. The
//! leaf is the KCC-1 Hash (BLAKE3-256) of that encoding.
//!
//! The attester is a KCC-2 `p2pk-schnorr/v1` authority: a 32-byte x-only key.
//! It signs [`attestation_message`] with KIP-5 personal-message signing, so any
//! Kaspa wallet that implements KIP-5 (Kaspire's `signMessage`) can attest.
//! The message is readable in the wallet and ends with the leaf, which commits
//! to every field.

use secp256k1::{Keypair, XOnlyPublicKey};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{hash::kcc_hash, hex, kip5, units, units::Unit};

pub const RECORD_DOMAIN: &[u8] = b"veritas.price.v1";
pub const MAX_ITEM_LEN: usize = 64;
pub const MAX_PLACE_LEN: usize = 128;

/// How far the attestation time can be trusted for this price.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Attested at or near the time the price was observed.
    Live,
    /// Price shown on a page captured by an independent archive; that
    /// archive's timestamp is the witness for `observed_on`.
    Archive,
    /// Attested after the fact (an old receipt). A YES proves only that the
    /// record existed by the anchoring time, not the printed date.
    AttestedLate,
}

impl Source {
    pub fn code(self) -> u8 {
        match self {
            Source::Live => 0,
            Source::Archive => 1,
            Source::AttestedLate => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Source::Live => "live",
            Source::Archive => "archive",
            Source::AttestedLate => "attested_late",
        }
    }
}

/// One observed price.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceRecord {
    /// Catalogue item id, e.g. `milk.whole.1l.dk`: lower-case ASCII letters,
    /// digits, `.`, `_` and `-`.
    pub item: String,
    /// Price in minor currency units (øre, cents).
    pub amount_minor: u64,
    /// ISO 4217 code, e.g. `DKK`.
    pub currency: String,
    /// Quantity as printed, in thousandths of `unit` (1 L = 1000).
    pub qty_milli: u64,
    pub unit: Unit,
    /// Shop and city, or a region only. May be empty.
    #[serde(default)]
    pub place: String,
    /// Date of purchase or shelf reading, `YYYY-MM-DD`.
    pub observed_on: String,
    pub source: Source,
    /// KCC-1 Hash (BLAKE3-256) of the receipt or shelf photo; zeros when there is none.
    #[serde(with = "hex::array")]
    pub evidence: [u8; 32],
    /// Random per-record value so private fields cannot be guessed from the leaf.
    #[serde(with = "hex::array")]
    pub salt: [u8; 16],
    /// Attester's 32-byte x-only public key.
    #[serde(with = "hex::array")]
    pub attester: [u8; 32],
}

/// A record plus the attester's KIP-5 signature over [`attestation_message`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedRecord {
    pub record: PriceRecord,
    #[serde(with = "hex::array")]
    pub signature: [u8; 64],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordError {
    InvalidItem,
    InvalidCurrency,
    ZeroQuantity,
    PlaceTooLong,
    InvalidDate,
    InvalidAttesterKey,
    KeyDoesNotMatchAttester,
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RecordError::InvalidItem => "item must be 1-64 chars of a-z, 0-9, '.', '_' or '-'",
            RecordError::InvalidCurrency => "currency must be a 3-letter upper-case ISO 4217 code",
            RecordError::ZeroQuantity => "quantity must be greater than zero",
            RecordError::PlaceTooLong => "place must be at most 128 bytes",
            RecordError::InvalidDate => "observed_on must be a real date in YYYY-MM-DD form",
            RecordError::InvalidAttesterKey => "attester is not a valid x-only public key",
            RecordError::KeyDoesNotMatchAttester => {
                "signing key does not match the record's attester"
            }
        })
    }
}

impl std::error::Error for RecordError {}

impl PriceRecord {
    pub fn validate(&self) -> Result<(), RecordError> {
        let item_ok = !self.item.is_empty()
            && self.item.len() <= MAX_ITEM_LEN
            && self.item.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            });
        if !item_ok {
            return Err(RecordError::InvalidItem);
        }
        if self.currency.len() != 3 || !self.currency.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(RecordError::InvalidCurrency);
        }
        if self.qty_milli == 0 {
            return Err(RecordError::ZeroQuantity);
        }
        if self.place.len() > MAX_PLACE_LEN {
            return Err(RecordError::PlaceTooLong);
        }
        if !is_valid_date(&self.observed_on) {
            return Err(RecordError::InvalidDate);
        }
        Ok(())
    }

    /// The canonical byte encoding that the leaf hash commits to.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RecordError> {
        self.validate()?;
        let mut out = Vec::with_capacity(160 + self.item.len() + self.place.len());
        push_lp(&mut out, RECORD_DOMAIN);
        push_lp(&mut out, self.item.as_bytes());
        out.extend_from_slice(&self.amount_minor.to_le_bytes());
        out.extend_from_slice(self.currency.as_bytes());
        out.extend_from_slice(&self.qty_milli.to_le_bytes());
        push_lp(&mut out, self.unit.code().as_bytes());
        push_lp(&mut out, self.place.as_bytes());
        out.extend_from_slice(self.observed_on.as_bytes());
        out.push(self.source.code());
        out.extend_from_slice(&self.evidence);
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&self.attester);
        Ok(out)
    }

    /// KCC-1 Hash (BLAKE3-256) of the canonical encoding.
    pub fn leaf(&self) -> Result<[u8; 32], RecordError> {
        Ok(kcc_hash(&[&self.canonical_bytes()?]))
    }

    /// Price per litre, kilogram or item in thousandths of the minor unit.
    pub fn price_per_base_milli(&self) -> Option<u128> {
        units::price_per_base_milli(self.amount_minor, self.qty_milli, self.unit)
    }

    /// The exact text the attester signs with KIP-5 (shown by the wallet).
    pub fn attestation_message(&self) -> Result<String, RecordError> {
        let leaf = self.leaf()?;
        Ok(attestation_message(self, &leaf))
    }

    /// Signs the attestation message with KIP-5. The key must be `self.attester`.
    pub fn sign(self, keypair: &Keypair) -> Result<SignedRecord, RecordError> {
        if keypair.x_only_public_key().0.serialize() != self.attester {
            return Err(RecordError::KeyDoesNotMatchAttester);
        }
        let signature = kip5::sign(self.attestation_message()?.as_bytes(), keypair);
        Ok(SignedRecord {
            record: self,
            signature,
        })
    }
}

/// The attestation text: a readable summary for the wallet's review screen,
/// ending with the leaf. Verifiers rebuild it from the record, so the summary
/// cannot disagree with what was signed.
pub fn attestation_message(record: &PriceRecord, leaf: &[u8; 32]) -> String {
    format!(
        "Veritas price attestation v1\nitem: {}\namount_minor: {} {}\nqty_milli: {} {}\nobserved_on: {} ({})\nleaf: {}",
        record.item,
        record.amount_minor,
        record.currency,
        record.qty_milli,
        record.unit.code(),
        record.observed_on,
        record.source.name(),
        hex::encode(leaf)
    )
}

impl SignedRecord {
    /// Returns the leaf if the KIP-5 signature is valid for the record's attester.
    pub fn verified_leaf(&self) -> Result<Option<[u8; 32]>, RecordError> {
        let leaf = self.record.leaf()?;
        XOnlyPublicKey::from_slice(&self.record.attester)
            .map_err(|_| RecordError::InvalidAttesterKey)?;
        let message = attestation_message(&self.record, &leaf);
        Ok(
            kip5::verify(message.as_bytes(), &self.signature, &self.record.attester)
                .then_some(leaf),
        )
    }
}

fn push_lp(out: &mut Vec<u8>, value: &[u8]) {
    // Lengths are bounded by validation, so the cast cannot truncate.
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

fn is_valid_date(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = &text[range];
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    let (Some(year), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10)) else {
        return false;
    };
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year >= 1900 && (1..=days).contains(&day)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use secp256k1::{Secp256k1, SecretKey};

    pub fn test_keypair(byte: u8) -> Keypair {
        let secret = SecretKey::from_slice(&[byte; 32]).expect("valid test key");
        Keypair::from_secret_key(&Secp256k1::new(), &secret)
    }

    pub fn milk(keypair: &Keypair, amount_minor: u64, observed_on: &str) -> PriceRecord {
        PriceRecord {
            item: "milk.whole.1l.dk".into(),
            amount_minor,
            currency: "DKK".into(),
            qty_milli: 1_000,
            unit: Unit::Litre,
            place: "Copenhagen".into(),
            observed_on: observed_on.into(),
            source: Source::Live,
            evidence: [0; 32],
            salt: [7; 16],
            attester: keypair.x_only_public_key().0.serialize(),
        }
    }

    #[test]
    fn canonical_layout_is_stable() {
        let record = milk(&test_keypair(1), 1_295, "2026-10-10");
        let bytes = record.canonical_bytes().unwrap();
        let expected = [
            &[16, 0, 0, 0][..],
            b"veritas.price.v1",
            &[16, 0, 0, 0],
            b"milk.whole.1l.dk",
            &1_295u64.to_le_bytes(),
            b"DKK",
            &1_000u64.to_le_bytes(),
            &[1, 0, 0, 0],
            b"l",
            &[10, 0, 0, 0],
            b"Copenhagen",
            b"2026-10-10",
            &[0],
            &[0; 32],
            &[7; 16],
            &record.attester,
        ]
        .concat();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn any_change_changes_the_leaf() {
        let keypair = test_keypair(1);
        let base = milk(&keypair, 1_295, "2026-10-10");
        let mut edited = base.clone();
        edited.amount_minor = 1_296;
        assert_ne!(base.leaf().unwrap(), edited.leaf().unwrap());
        let mut resalted = base.clone();
        resalted.salt[0] ^= 1;
        assert_ne!(base.leaf().unwrap(), resalted.leaf().unwrap());
    }

    #[test]
    fn signature_round_trips_and_detects_tampering() {
        let keypair = test_keypair(1);
        let signed = milk(&keypair, 1_295, "2026-10-10").sign(&keypair).unwrap();
        assert!(signed.verified_leaf().unwrap().is_some());

        let mut tampered = signed.clone();
        tampered.record.amount_minor = 995;
        assert_eq!(tampered.verified_leaf().unwrap(), None);
    }

    #[test]
    fn wallet_message_is_readable_and_ends_with_the_leaf() {
        let record = milk(&test_keypair(1), 1_295, "2026-10-10");
        let message = record.attestation_message().unwrap();
        let leaf = crate::hex::encode(&record.leaf().unwrap());
        assert_eq!(
            message,
            format!(
                "Veritas price attestation v1\nitem: milk.whole.1l.dk\namount_minor: 1295 DKK\nqty_milli: 1000 l\nobserved_on: 2026-10-10 (live)\nleaf: {leaf}"
            )
        );
    }

    #[test]
    fn a_wallet_kip5_signature_is_accepted() {
        // What Kaspire's signMessage does: KIP-5 over the attestation text.
        let keypair = test_keypair(1);
        let record = milk(&keypair, 1_295, "2026-10-10");
        let signature = kip5::sign(record.attestation_message().unwrap().as_bytes(), &keypair);
        let signed = SignedRecord { record, signature };
        assert!(signed.verified_leaf().unwrap().is_some());
    }

    #[test]
    fn refuses_to_sign_for_someone_else() {
        let record = milk(&test_keypair(1), 1_295, "2026-10-10");
        assert_eq!(
            record.sign(&test_keypair(2)),
            Err(RecordError::KeyDoesNotMatchAttester)
        );
    }

    #[test]
    fn validation_rejects_bad_fields() {
        let keypair = test_keypair(1);
        type Case = (fn(&mut PriceRecord), RecordError);
        let cases: [Case; 6] = [
            (|r| r.item = "Milk".into(), RecordError::InvalidItem),
            (|r| r.currency = "dkk".into(), RecordError::InvalidCurrency),
            (|r| r.qty_milli = 0, RecordError::ZeroQuantity),
            (|r| r.place = "x".repeat(129), RecordError::PlaceTooLong),
            (
                |r| r.observed_on = "2026-02-29".into(),
                RecordError::InvalidDate,
            ),
            (
                |r| r.observed_on = "2026-13-01".into(),
                RecordError::InvalidDate,
            ),
        ];
        for (mutate, expected) in cases {
            let mut record = milk(&keypair, 1_295, "2026-10-10");
            mutate(&mut record);
            assert_eq!(record.validate(), Err(expected));
        }
        let mut leap = milk(&keypair, 1_295, "2028-02-29");
        assert_eq!(leap.validate(), Ok(()));
        leap.observed_on = "2100-02-29".into();
        assert_eq!(leap.validate(), Err(RecordError::InvalidDate));
    }

    #[test]
    fn json_uses_unit_codes_and_hex() {
        let record = milk(&test_keypair(1), 1_295, "2026-10-10");
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["unit"], "l");
        assert_eq!(json["source"], "live");
        assert_eq!(json["salt"], "07070707070707070707070707070707");
        let back: PriceRecord = serde_json::from_value(json).unwrap();
        assert_eq!(back, record);
    }
}
