//! `coded_error.v1` as hand-written prost types. Consumers `extern_path`
//! `.coded_error.v1` to `::coded_error::proto::v1`.
pub mod v1 {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ErrorInfo {
        #[prost(string, tag = "1")]
        pub code: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub class: ::prost::alloc::string::String,
        #[prost(string, tag = "3")]
        pub fingerprint: ::prost::alloc::string::String,
    }

    impl From<&crate::ErrorInfo> for ErrorInfo {
        fn from(i: &crate::ErrorInfo) -> Self {
            ErrorInfo {
                code: i.code.clone(),
                class: i.class.as_str().to_owned(),
                fingerprint: i.fingerprint.clone(),
            }
        }
    }
}
