use coded_error::{fingerprint, ErrorClass};

#[derive(serde::Deserialize)]
struct Vector {
    code: String,
    class: String,
    source: String,
    fingerprint: String,
}

#[test]
fn matches_golden_vectors() {
    let raw = include_str!("../../vectors/fingerprint.json");
    let vectors: Vec<Vector> = serde_json::from_str(raw).unwrap();
    assert!(vectors.len() >= 6);
    for v in vectors {
        let class: ErrorClass = v.class.parse().unwrap();
        assert_eq!(
            fingerprint(&v.code, class, &v.source),
            v.fingerprint,
            "{}",
            v.code
        );
    }
}

fn hex8(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn separators_are_single_zero_bytes() {
    let got = fingerprint("a.b.c", ErrorClass::Logical, "x");
    // Exactly code 0x00 class 0x00 source.
    assert_eq!(got, hex8(b"a.b.c\0logical\0x"));
    // Not the unseparated concatenation.
    assert_ne!(got, hex8(b"a.b.clogicalx"));
}
