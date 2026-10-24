use coded_error::{check_codes, is_valid_code, ErrorClass};

#[test]
fn grammar_accepts_three_or_more_segments() {
    for ok in [
        "grorg.action.not_found",
        "structmux.gate.denied",
        "a.b.c",
        "concord.process.panic",
        "a1.b_2.c3.d",
    ] {
        assert!(is_valid_code(ok), "{ok}");
    }
}

#[test]
fn grammar_rejects_bad_codes() {
    for bad in [
        "",
        "unknown",
        "a.b",
        "A.b.c",
        "a..b.c",
        "a.b.c.",
        ".a.b.c",
        "1a.b.c",
        "a.b-c.d",
        "a.b.C",
        "a.b._c",
    ] {
        assert!(!is_valid_code(bad), "{bad}");
    }
}

#[test]
fn class_names_are_lowercase_and_round_trip() {
    for (c, s) in [
        (ErrorClass::Logical, "logical"),
        (ErrorClass::Infrastructure, "infrastructure"),
        (ErrorClass::Transient, "transient"),
        (ErrorClass::Unknown, "unknown"),
    ] {
        assert_eq!(c.as_str(), s);
        assert_eq!(s.parse::<ErrorClass>().unwrap(), c);
    }
    assert!("Logical".parse::<ErrorClass>().is_err());
}

#[test]
fn check_codes_enforces_prefix_and_uniqueness() {
    assert!(check_codes("grorg.action.", &[&["grorg.action.a", "grorg.action.b"]]).is_ok());
    let dup = check_codes("grorg.", &[&["grorg.action.a"], &["grorg.action.a"]]).unwrap_err();
    assert!(dup.contains("duplicate"), "{dup}");
    let pre = check_codes("grorg.", &[&["multimatrix.room.x"]]).unwrap_err();
    assert!(pre.contains("prefix"), "{pre}");
    let bad = check_codes("grorg.", &[&["grorg.x"]]).unwrap_err();
    assert!(bad.contains("grammar"), "{bad}");
}
