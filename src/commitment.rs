use std::fmt;

use blake2b_simd::Params;

pub const COMMITMENT_DOMAIN: &[u8] = b"veritas.colp.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteAnswer {
    pub region_id: String,
    pub necessities_pct: u8,
    pub employed: bool,
    pub duration_months: u32,
    pub month_key: String,
    pub wallet: String,
    pub salt: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitmentError {
    InvalidNecessitiesPct(u8),
    InvalidMonthKey,
    EmptyRegionId,
    NonAsciiRegionId,
    EmptyWallet,
    FieldTooLong(&'static str),
}

impl fmt::Display for CommitmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNecessitiesPct(value) => {
                write!(
                    formatter,
                    "necessities percentage must be at most 100, got {value}"
                )
            }
            Self::InvalidMonthKey => formatter.write_str("month key must match YYYY-MM"),
            Self::EmptyRegionId => formatter.write_str("region ID must not be empty"),
            Self::NonAsciiRegionId => formatter.write_str("region ID must contain only ASCII"),
            Self::EmptyWallet => formatter.write_str("wallet must not be empty"),
            Self::FieldTooLong(field) => {
                write!(formatter, "{field} is too long for canonical encoding")
            }
        }
    }
}

impl std::error::Error for CommitmentError {}

/// Encodes a vote answer in the protocol's canonical byte layout.
pub fn canonical_bytes(answer: &VoteAnswer) -> Result<Vec<u8>, CommitmentError> {
    validate(answer)?;

    let mut output = Vec::new();
    push_length_prefixed(&mut output, COMMITMENT_DOMAIN, "commitment domain")?;
    push_length_prefixed(&mut output, answer.region_id.as_bytes(), "region ID")?;
    output.push(answer.necessities_pct);
    output.push(u8::from(answer.employed));
    output.extend_from_slice(&answer.duration_months.to_le_bytes());
    push_length_prefixed(&mut output, answer.month_key.as_bytes(), "month key")?;
    push_length_prefixed(&mut output, answer.wallet.as_bytes(), "wallet")?;
    output.extend_from_slice(&answer.salt);
    Ok(output)
}

/// Returns the BLAKE2b-256 digest of the canonical vote encoding.
pub fn commitment_hash(answer: &VoteAnswer) -> Result<[u8; 32], CommitmentError> {
    let bytes = canonical_bytes(answer)?;
    let hash = Params::new().hash_length(32).hash(&bytes);
    let mut commitment = [0; 32];
    commitment.copy_from_slice(hash.as_bytes());
    Ok(commitment)
}

fn validate(answer: &VoteAnswer) -> Result<(), CommitmentError> {
    if answer.necessities_pct > 100 {
        return Err(CommitmentError::InvalidNecessitiesPct(
            answer.necessities_pct,
        ));
    }
    if answer.region_id.is_empty() {
        return Err(CommitmentError::EmptyRegionId);
    }
    if !answer.region_id.is_ascii() {
        return Err(CommitmentError::NonAsciiRegionId);
    }
    if !is_month_key(&answer.month_key) {
        return Err(CommitmentError::InvalidMonthKey);
    }
    if answer.wallet.is_empty() {
        return Err(CommitmentError::EmptyWallet);
    }
    Ok(())
}

fn is_month_key(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 7
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..].iter().all(u8::is_ascii_digit)
}

fn push_length_prefixed(
    output: &mut Vec<u8>,
    value: &[u8],
    field: &'static str,
) -> Result<(), CommitmentError> {
    let length = u32::try_from(value.len()).map_err(|_| CommitmentError::FieldTooLong(field))?;
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn golden_answer(employed: bool) -> VoteAnswer {
        VoteAnswer {
            region_id: "euro-area".into(),
            necessities_pct: 42,
            employed,
            duration_months: 18,
            month_key: "2026-08".into(),
            wallet: "kaspatest:qqveritasgoldenwallet1".into(),
            salt: [7; 16],
        }
    }

    #[test]
    fn golden_commitment_vector_1() {
        let answer = golden_answer(true);

        let bytes = canonical_bytes(&answer).unwrap();
        let expected = [
            &[15, 0, 0, 0][..],
            b"veritas.colp.v1",
            &[9, 0, 0, 0],
            b"euro-area",
            &[42, 1],
            &[18, 0, 0, 0],
            &[7, 0, 0, 0],
            b"2026-08",
            &[32, 0, 0, 0],
            b"kaspatest:qqveritasgoldenwallet1",
            &[7; 16],
        ]
        .concat();
        assert_eq!(bytes, expected);

        let hash = commitment_hash(&answer).unwrap();
        assert_eq!(
            hex_encode(&hash),
            "ebbca110bc4f0c17b38ea1b8e5a447e37a77ebc80a9192003fc10bb5e97c1dcb"
        );
    }

    #[test]
    fn golden_commitment_vector_2() {
        let answer = golden_answer(false);

        let bytes = canonical_bytes(&answer).unwrap();
        assert_eq!(bytes[33], 0);
        let hash = commitment_hash(&answer).unwrap();
        assert_eq!(
            hex_encode(&hash),
            "a7d78af0d17c3abeb78582cd812304d146305821943bb24788cd9e42c7600a0d"
        );
    }

    #[test]
    fn rejects_invalid_answers() {
        let mut answer = golden_answer(true);
        answer.necessities_pct = 101;
        assert_eq!(
            canonical_bytes(&answer),
            Err(CommitmentError::InvalidNecessitiesPct(101))
        );

        answer = golden_answer(true);
        answer.month_key = "2026-8".into();
        assert_eq!(
            canonical_bytes(&answer),
            Err(CommitmentError::InvalidMonthKey)
        );

        answer = golden_answer(true);
        answer.region_id.clear();
        assert_eq!(
            canonical_bytes(&answer),
            Err(CommitmentError::EmptyRegionId)
        );

        answer = golden_answer(true);
        answer.region_id = "europé".into();
        assert_eq!(
            canonical_bytes(&answer),
            Err(CommitmentError::NonAsciiRegionId)
        );

        answer = golden_answer(true);
        answer.wallet.clear();
        assert_eq!(canonical_bytes(&answer), Err(CommitmentError::EmptyWallet));
    }

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
