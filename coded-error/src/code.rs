use std::collections::HashSet;

/// The reserved code for errors that have no code yet. It does not match
/// the grammar; consumers reading `ErrorInfo.code` special-case it.
pub const UNKNOWN_CODE: &str = "unknown";

/// `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*){2,}$`. `const` so the derive can
/// assert it at compile time without a second implementation.
pub const fn is_valid_code(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() {
        return false;
    }
    let mut i = 0;
    let mut segments = 1;
    let mut at_segment_start = true;
    while i < b.len() {
        let c = b[i];
        if at_segment_start {
            if !c.is_ascii_lowercase() {
                return false;
            }
            at_segment_start = false;
        } else if c == b'.' {
            segments += 1;
            at_segment_start = true;
        } else if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_') {
            return false;
        }
        i += 1;
    }
    !at_segment_start && segments >= 3
}

/// The one implementation behind every repo's per-crate and aggregate
/// uniqueness tests: each code matches the grammar, starts with `prefix`,
/// and appears once across all `sets` (each set is an enum's `ALL_CODES`).
/// The `concord.` prefix is reserved for coded-error and concord-telemetry;
/// it is not enforced here, so other repos must not use it.
pub fn check_codes(prefix: &str, sets: &[&[&str]]) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut problems = Vec::new();
    for code in sets.iter().flat_map(|s| s.iter()) {
        if !is_valid_code(code) {
            problems.push(format!("{code}: does not match the code grammar"));
        }
        if !code.starts_with(prefix) {
            problems.push(format!("{code}: missing prefix {prefix:?}"));
        }
        if !seen.insert(*code) {
            problems.push(format!("{code}: duplicate"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}
