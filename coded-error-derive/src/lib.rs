use proc_macro::TokenStream;

#[proc_macro_derive(CodedError, attributes(coded))]
pub fn derive_coded_error(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
