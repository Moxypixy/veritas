//! Lower-case hex helpers and serde adapters for fixed-size byte arrays.

use std::fmt::Write as _;

/// Encodes bytes as lower-case hex.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Decodes hex into exactly `N` bytes.
pub fn decode_array<const N: usize>(text: &str) -> Result<[u8; N], String> {
    let text = text.trim();
    if text.len() != N * 2 {
        return Err(format!(
            "expected {} hex characters, got {}",
            N * 2,
            text.len()
        ));
    }
    let mut out = [0; N];
    for (index, chunk) in text.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(chunk).map_err(|_| "hex must be ASCII".to_string())?;
        out[index] =
            u8::from_str_radix(pair, 16).map_err(|_| format!("invalid hex pair `{pair}`"))?;
    }
    Ok(out)
}

/// Serde adapter: `[u8; N]` as a hex string.
pub mod array {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer, const N: usize>(
        value: &[u8; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&super::encode(value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<[u8; N], D::Error> {
        let text = String::deserialize(deserializer)?;
        super::decode_array(&text).map_err(serde::de::Error::custom)
    }
}

/// Serde adapter: `Vec<[u8; 32]>` as a list of hex strings.
pub mod array_list {
    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeSeq};

    pub fn serialize<S: Serializer>(values: &[[u8; 32]], serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(values.len()))?;
        for value in values {
            seq.serialize_element(&super::encode(value))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<[u8; 32]>, D::Error> {
        let texts = Vec::<String>::deserialize(deserializer)?;
        texts
            .iter()
            .map(|text| super::decode_array(text).map_err(serde::de::Error::custom))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let bytes = [0x00, 0x0f, 0xa5, 0xff];
        assert_eq!(encode(&bytes), "000fa5ff");
        assert_eq!(decode_array::<4>("000FA5ff").unwrap(), bytes);
    }

    #[test]
    fn rejects_wrong_length_and_bad_digits() {
        assert!(decode_array::<2>("abc").is_err());
        assert!(decode_array::<2>("zz00").is_err());
    }
}
