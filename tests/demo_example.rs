//! The committed demo in `examples/demo` must keep verifying. The web verifier
//! tests read the same files, so Rust and TypeScript stay in agreement.

use std::{fs, path::PathBuf};

use veritas::{AnchorLog, NoReason, Receipt, SignedRecord, Verdict, hex, verify};

fn demo(path: &str) -> String {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/demo")
        .join(path);
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}

fn load(name: &str) -> (SignedRecord, Receipt) {
    (
        serde_json::from_str(&demo(&format!("{name}.signed.json"))).unwrap(),
        serde_json::from_str(&demo(&format!("receipts/{name}.receipt.json"))).unwrap(),
    )
}

fn log_and_acc() -> (AnchorLog, [u8; 32]) {
    let log: AnchorLog = serde_json::from_str(&demo("anchor-log.json")).unwrap();
    let acc = hex::decode_array(&demo("live-acc.txt")).unwrap();
    (log, acc)
}

#[test]
fn every_demo_record_verifies_yes() {
    let (log, acc) = log_and_acc();
    assert_eq!(log.acc(), acc);
    for name in ["milk-2026-03", "milk-us-gallon", "milk-2026-10"] {
        let (signed, receipt) = load(name);
        assert!(
            matches!(
                verify(&signed, &receipt, &log.batch_roots, &acc),
                Verdict::Yes(_)
            ),
            "{name}"
        );
    }
}

#[test]
fn demo_shows_the_price_change_and_unit_conversion() {
    let (log, acc) = log_and_acc();
    let price = |name| match verify(&load(name).0, &load(name).1, &log.batch_roots, &acc) {
        Verdict::Yes(c) => c.price_per_base_milli.unwrap(),
        Verdict::No(reason) => panic!("{name}: {reason}"),
    };
    assert_eq!(price("milk-2026-03"), 1_195_000);
    assert_eq!(price("milk-2026-10"), 1_295_000);
    assert_eq!(price("milk-us-gallon"), 113_858);
}

#[test]
fn editing_one_digit_of_the_demo_says_no() {
    let (log, acc) = log_and_acc();
    let (mut signed, receipt) = load("milk-2026-03");
    signed.record.amount_minor = 995;
    assert_eq!(
        verify(&signed, &receipt, &log.batch_roots, &acc),
        Verdict::No(NoReason::BadSignature)
    );
}
