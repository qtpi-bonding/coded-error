use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod expand;

#[proc_macro_derive(CodedError, attributes(coded))]
/// Derive `CodedError` for an enum using `#[coded(...)]` attributes: set
/// `namespace` on the enum; on each variant set `code`, exactly one of
/// `class = X`, `class = from(field)`, `class_with = path`, or `transparent`,
/// plus optional `grpc`, `http`, `mcp` and `acp` overrides.
pub fn derive_coded_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
