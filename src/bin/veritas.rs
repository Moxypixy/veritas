//! `veritas` command-line tool for the v0 demo.
//!
//! ```text
//! veritas keygen <key-file>
//! veritas attest <key-file> <draft.json> <signed.json>
//! veritas batch  <anchor-log.json> <out-dir> <signed.json>...
//! veritas verify <signed.json> <receipt.json> <anchor-log.json> <live-acc-hex>
//! veritas unit-price <signed.json> [display-unit]
//! ```
//!
//! `attest` reads a draft record (no salt or attester needed), fills in a fresh
//! salt and the key's public key, and signs it. `batch` builds the Merkle tree,
//! appends the root to the anchor log, writes one receipt per record and
//! prints the accumulator the anchor covenant must move to. Broadcasting that
//! covenant transaction is done by the anchoring service (not in v0).

use std::{fs, path::Path, process::ExitCode};

use secp256k1::{Keypair, Secp256k1, SecretKey};
use serde::Deserialize;
use veritas::{
    AnchorLog, Batch, PriceRecord, Receipt, SignedRecord, Source, Unit, Verdict, hex,
    receipt::RECEIPT_VERSION,
};

type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

const USAGE: &str = "usage:
  veritas keygen <key-file>
  veritas attest <key-file> <draft.json> <signed.json>
  veritas batch <anchor-log.json> <out-dir> <signed.json>...
  veritas verify <signed.json> <receipt.json> <anchor-log.json> <live-acc-hex>
  veritas unit-price <signed.json> [display-unit, e.g. l, us_gal, imp_pt]";

/// A record before attestation: salt and attester are filled in by `attest`.
#[derive(Deserialize)]
struct Draft {
    item: String,
    amount_minor: u64,
    currency: String,
    qty_milli: u64,
    unit: Unit,
    #[serde(default)]
    place: String,
    observed_on: String,
    source: Source,
    #[serde(default, with = "hex_opt")]
    evidence: Option<[u8; 32]>,
}

mod hex_opt {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<[u8; 32]>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|text| veritas::hex::decode_array(&text).map_err(serde::de::Error::custom))
            .transpose()
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("keygen") if args.len() == 2 => keygen(&args[1]),
        Some("attest") if args.len() == 4 => attest(&args[1], &args[2], &args[3]),
        Some("batch") if args.len() >= 4 => batch(&args[1], &args[2], &args[3..]),
        Some("verify") if args.len() == 5 => verify(&args[1], &args[2], &args[3], &args[4]),
        Some("unit-price") if args.len() == 2 || args.len() == 3 => {
            unit_price(&args[1], args.get(2).map(String::as_str))
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn random<const N: usize>() -> CliResult<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|e| format!("no system randomness: {e}"))?;
    Ok(bytes)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> CliResult<T> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> CliResult<()> {
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(())
}

fn load_keypair(path: &str) -> CliResult<Keypair> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let secret = SecretKey::from_slice(&hex::decode_array::<32>(&text)?)?;
    Ok(Keypair::from_secret_key(&Secp256k1::new(), &secret))
}

fn keygen(path: &str) -> CliResult<ExitCode> {
    if Path::new(path).exists() {
        return Err(format!("{path} already exists; refusing to overwrite a key").into());
    }
    let secret = SecretKey::from_slice(&random::<32>()?)?;
    fs::write(path, hex::encode(&secret.secret_bytes()) + "\n")?;
    let keypair = Keypair::from_secret_key(&Secp256k1::new(), &secret);
    println!(
        "attester {}",
        hex::encode(&keypair.x_only_public_key().0.serialize())
    );
    println!("secret key written to {path}; keep it private (testnet use only)");
    Ok(ExitCode::SUCCESS)
}

fn attest(key_path: &str, draft_path: &str, out_path: &str) -> CliResult<ExitCode> {
    let keypair = load_keypair(key_path)?;
    let draft: Draft = read_json(draft_path)?;
    let record = PriceRecord {
        item: draft.item,
        amount_minor: draft.amount_minor,
        currency: draft.currency,
        qty_milli: draft.qty_milli,
        unit: draft.unit,
        place: draft.place,
        observed_on: draft.observed_on,
        source: draft.source,
        evidence: draft.evidence.unwrap_or([0; 32]),
        salt: random()?,
        attester: keypair.x_only_public_key().0.serialize(),
    };
    let signed = record.sign(&keypair)?;
    write_json(Path::new(out_path), &signed)?;
    println!("leaf {}", hex::encode(&signed.record.leaf()?));
    Ok(ExitCode::SUCCESS)
}

fn batch(log_path: &str, out_dir: &str, record_paths: &[String]) -> CliResult<ExitCode> {
    let mut log: AnchorLog = if Path::new(log_path).exists() {
        read_json(log_path)?
    } else {
        AnchorLog::default()
    };
    let mut leaves = Vec::with_capacity(record_paths.len());
    for path in record_paths {
        let signed: SignedRecord = read_json(path)?;
        let leaf = signed
            .verified_leaf()?
            .ok_or_else(|| format!("{path}: signature is not valid"))?;
        leaves.push(leaf);
    }
    let batch = Batch::new(&leaves)?;
    let previous_acc = log.acc();
    let (batch_index, new_acc) = log.append(batch.root());
    fs::create_dir_all(out_dir)?;
    for (leaf_index, path) in record_paths.iter().enumerate() {
        let receipt = Receipt {
            version: RECEIPT_VERSION,
            batch_index,
            batch_root: batch.root(),
            leaf_index: leaf_index as u64,
            proof: batch.proof(leaf_index)?,
            anchor_txid: None,
        };
        let stem = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("record");
        let stem = stem.strip_suffix(".signed").unwrap_or(stem);
        write_json(
            &Path::new(out_dir).join(format!("{stem}.receipt.json")),
            &receipt,
        )?;
    }
    write_json(Path::new(log_path), &log)?;
    println!("batch {batch_index}: {} record(s)", record_paths.len());
    println!("batch root  {}", hex::encode(&batch.root()));
    println!("anchor from {}", hex::encode(&previous_acc));
    println!("anchor to   {}", hex::encode(&new_acc));
    Ok(ExitCode::SUCCESS)
}

fn verify(
    signed_path: &str,
    receipt_path: &str,
    log_path: &str,
    live_acc_hex: &str,
) -> CliResult<ExitCode> {
    let signed: SignedRecord = read_json(signed_path)?;
    let receipt: Receipt = read_json(receipt_path)?;
    let log: AnchorLog = read_json(log_path)?;
    let live_acc = hex::decode_array::<32>(live_acc_hex)?;
    match veritas::verify(&signed, &receipt, &log.batch_roots, &live_acc) {
        Verdict::Yes(confirmation) => {
            let record = &signed.record;
            println!("YES");
            println!(
                "{} {} {} for {} {} on {}",
                record.item,
                record.amount_minor,
                record.currency,
                format_qty(record.qty_milli),
                record.unit.code(),
                record.observed_on
            );
            println!("attester {}", hex::encode(&confirmation.attester));
            println!("batch {} of the anchor log", confirmation.batch_index);
            println!("{}", confirmation.time_claim());
            Ok(ExitCode::SUCCESS)
        }
        Verdict::No(reason) => {
            println!("NO: {reason}");
            Ok(ExitCode::FAILURE)
        }
    }
}

fn unit_price(signed_path: &str, display: Option<&str>) -> CliResult<ExitCode> {
    let signed: SignedRecord = read_json(signed_path)?;
    let record = &signed.record;
    let base = match record.unit.dimension() {
        veritas::Dimension::Volume => Unit::Litre,
        veritas::Dimension::Mass => Unit::Kilogram,
        veritas::Dimension::Count => Unit::Each,
    };
    let display = match display {
        Some(code) => Unit::from_code(code).ok_or_else(|| format!("unknown unit `{code}`"))?,
        None => base,
    };
    let milli = veritas::units::price_per_unit_milli(
        record.amount_minor,
        record.qty_milli,
        record.unit,
        display,
    )
    .ok_or_else(|| {
        format!(
            "cannot show a price per {} for a record measured in {}",
            display.code(),
            record.unit.code()
        )
    })?;
    println!(
        "{}.{:03} {} minor units per {}",
        milli / 1000,
        milli % 1000,
        record.currency,
        display.code()
    );
    Ok(ExitCode::SUCCESS)
}

fn format_qty(qty_milli: u64) -> String {
    if qty_milli.is_multiple_of(1000) {
        (qty_milli / 1000).to_string()
    } else {
        format!("{}.{:03}", qty_milli / 1000, qty_milli % 1000)
            .trim_end_matches('0')
            .to_string()
    }
}
