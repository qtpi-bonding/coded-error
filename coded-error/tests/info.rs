#![cfg(feature = "serde")]
use coded_error::{ErrorClass, ErrorInfo, Facts, Overrides};

#[test]
fn json_shape_matches_proto3_json() {
    let facts = Facts {
        code: "grorg.action.not_found",
        class: ErrorClass::Logical,
        overrides: Overrides::NONE,
    };
    let info = ErrorInfo::from_facts(&facts, "grorg_server::GetAction");
    let v = serde_json::to_value(&info).unwrap();
    assert_eq!(
        v,
        serde_json::json!({
            "code": "grorg.action.not_found",
            "class": "logical",
            "fingerprint": "4ad3d16c4463edee"
        })
    );
    let back: ErrorInfo = serde_json::from_value(v).unwrap();
    assert_eq!(back, info);
}
