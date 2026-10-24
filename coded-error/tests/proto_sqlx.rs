#[cfg(feature = "proto")]
#[test]
fn proto_field_numbers_and_round_trip() {
    use prost::Message;
    let info = coded_error::ErrorInfo {
        code: "a.b.c".into(),
        class: coded_error::ErrorClass::Transient,
        fingerprint: "0011223344556677".into(),
    };
    let p = coded_error::proto::v1::ErrorInfo::from(&info);
    let bytes = p.encode_to_vec();
    // field 1 (tag 0x0a) "a.b.c", field 2 (0x12) "transient", field 3 (0x1a) fingerprint
    assert_eq!(&bytes[..2], &[0x0a, 5]);
    assert!(bytes.windows(11).any(|w| w == b"\x12\x09transient"));
    assert_eq!(
        coded_error::proto::v1::ErrorInfo::decode(&bytes[..]).unwrap(),
        p
    );
    let proto_file = include_str!("../../proto/coded_error/v1/error_info.proto");
    for line in [
        "string code = 1;",
        "string class = 2;",
        "string fingerprint = 3;",
    ] {
        assert!(proto_file.contains(line), "{line}");
    }
}

#[cfg(feature = "sqlx")]
#[test]
fn sqlx_classification() {
    use coded_error::{sqlx::classify_sqlx, ErrorClass};
    assert_eq!(
        classify_sqlx(&sqlx_core::Error::RowNotFound),
        ("row_not_found", ErrorClass::Logical)
    );
    assert_eq!(
        classify_sqlx(&sqlx_core::Error::PoolTimedOut),
        ("pool_timeout", ErrorClass::Transient)
    );
    assert_eq!(
        classify_sqlx(&sqlx_core::Error::PoolClosed).1,
        ErrorClass::Infrastructure
    );
}

#[cfg(feature = "sqlx")]
mod fake_db {
    use coded_error::{sqlx::classify_sqlx, ErrorClass};
    use sqlx_core::error::{DatabaseError, ErrorKind};

    #[derive(Debug)]
    struct Fake {
        unique: bool,
    }

    impl std::fmt::Display for Fake {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("fake db error")
        }
    }
    impl std::error::Error for Fake {}

    impl DatabaseError for Fake {
        fn message(&self) -> &str {
            "fake db error"
        }
        fn kind(&self) -> ErrorKind {
            if self.unique {
                ErrorKind::UniqueViolation
            } else {
                ErrorKind::Other
            }
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
    }

    #[test]
    fn database_error_kinds() {
        let unique = sqlx_core::Error::Database(Box::new(Fake { unique: true }));
        assert_eq!(
            classify_sqlx(&unique),
            ("unique_violation", ErrorClass::Logical)
        );
        let other = sqlx_core::Error::Database(Box::new(Fake { unique: false }));
        assert_eq!(
            classify_sqlx(&other),
            ("database", ErrorClass::Infrastructure)
        );
    }
}
