use crate::ErrorClass;
use sha2::{Digest, Sha256};

/// Groups errors. Not a security value. Excludes message, file, line and
/// backtrace (D12); `source` is the report-site label.
pub fn fingerprint(code: &str, class: ErrorClass, source: &str) -> String {
    let mut h = Sha256::new();
    h.update(code.as_bytes());
    h.update([0u8]);
    h.update(class.as_str().as_bytes());
    h.update([0u8]);
    h.update(source.as_bytes());
    let digest = h.finalize();
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}
