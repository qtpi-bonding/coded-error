use coded_error::{chain_anyhow, from_remote, into_anyhow, CodedError, ErrorClass, Origin};

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "x")]
enum TestError {
    #[error("io wrapper: {0}")]
    #[coded(code = "y.z", class = Infrastructure)]
    Io(#[source] std::io::Error),
}

#[test]
fn typed_error_with_io_source_is_own_then_external() {
    let e = TestError::Io(std::io::Error::other("disk"));
    let links = chain_anyhow(&into_anyhow(e));
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].code, Some("x.y.z"));
    assert_eq!(links[0].origin, Origin::Own);
    assert_eq!(links[0].type_name, Some(std::any::type_name::<TestError>()));
    assert_eq!(links[1].code, None);
    assert_eq!(links[1].origin, Origin::External);
    assert_eq!(links[1].message, "disk");
}

#[test]
fn anyhow_context_over_typed_is_external_then_own() {
    let e = TestError::Io(std::io::Error::other("disk"));
    let err = into_anyhow(e).context("while doing X");
    let links = chain_anyhow(&err);
    assert_eq!(links[0].message, "while doing X");
    assert_eq!(links[0].origin, Origin::External);
    assert_eq!(links[1].origin, Origin::Own);
}

#[test]
fn remote_status_is_external_with_its_code() {
    let err = from_remote(
        Some("x.remote.code"),
        Some(ErrorClass::Transient),
        "gone".into(),
    );
    let links = chain_anyhow(&err);
    assert_eq!(links[0].code, Some("x.remote.code"));
    assert_eq!(links[0].origin, Origin::External);
}
