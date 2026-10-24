use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{spanned::Spanned, Data, DeriveInput, Expr, ExprLit, Fields, Ident, Lit, LitInt, LitStr, Path};

enum ClassSource {
    Fixed(Ident),
    FromField(FieldRef),
    With(Path),
    Transparent,
}

enum FieldRef {
    Named(Ident),
    Index(usize),
}

struct VariantSpec {
    ident: Ident,
    fields: Fields,
    code: Option<LitStr>,
    class: ClassSource,
    grpc: Option<Ident>,
    http: Option<LitInt>,
    mcp: Option<Ident>,
}

pub fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(input.ident.span(), "CodedError derive supports enums only"));
    };
    let namespace = parse_namespace(input)?;
    let mut specs = Vec::new();
    let mut seen = HashSet::new();
    for v in &data.variants {
        let spec = parse_variant(v)?;
        if let Some(code) = &spec.code {
            if !seen.insert(code.value()) {
                return Err(syn::Error::new(code.span(), format!("duplicate code {:?}", code.value())));
            }
        }
        specs.push(spec);
    }
    if specs.is_empty() {
        return Err(syn::Error::new(input.ident.span(), "CodedError needs at least one variant"));
    }

    let name = &input.ident;
    let (impl_g, ty_g, where_g) = input.generics.split_for_impl();
    let full = |code: &LitStr| LitStr::new(&format!("{}.{}", namespace.value(), code.value()), code.span());
    let mut code_arms = Vec::new();
    let mut class_arms = Vec::new();
    let mut override_arms = Vec::new();
    let mut all_codes = Vec::new();
    let mut asserts = Vec::new();

    for s in &specs {
        let v = &s.ident;
        let own = overrides_tokens(s);
        if let Some(h) = &s.http {
            asserts.push(quote! { const _: () = ::core::assert!(#h >= 100 && #h <= 599, "http status must be 100..=599"); });
        }
        match &s.class {
            ClassSource::Transparent => {
                let inner = format_ident!("__inner");
                let pat = single_field_pattern(v, &s.fields, &inner)?;
                let (grpc, http, mcp) = override_parts(s);
                code_arms.push(quote! { #pat => ::coded_error::CodedError::code(#inner) });
                class_arms.push(quote! { #pat => ::coded_error::CodedError::class(#inner) });
                override_arms.push(quote! {
                    #pat => {
                        let __inner_overrides = ::coded_error::CodedError::overrides(#inner);
                        ::coded_error::Overrides {
                            grpc: #grpc.or(__inner_overrides.grpc),
                            http: #http.or(__inner_overrides.http),
                            mcp: #mcp.or(__inner_overrides.mcp),
                        }
                    }
                });
            }
            other => {
                let code = s.code.as_ref().expect("checked in parse_variant");
                let full_code = full(code);
                let msg = format!("code {:?} does not match the coded-error grammar", full_code.value());
                asserts.push(quote! { const _: () = ::core::assert!(::coded_error::is_valid_code(#full_code), #msg); });
                all_codes.push(full_code.clone());
                let wild = wildcard_pattern(v, &s.fields);
                code_arms.push(quote! { #wild => #full_code });
                override_arms.push(quote! { #wild => #own });
                let class = match other {
                    ClassSource::Fixed(c) => quote! { #wild => ::coded_error::ErrorClass::#c },
                    ClassSource::FromField(f) => {
                        let bind = format_ident!("__field");
                        let pat = field_pattern(v, &s.fields, f, &bind)?;
                        quote! { #pat => ::core::convert::Into::<::coded_error::ErrorClass>::into(::core::clone::Clone::clone(#bind)) }
                    }
                    ClassSource::With(path) => quote! { #wild => #path(self) },
                    ClassSource::Transparent => unreachable!(),
                };
                class_arms.push(class);
            }
        }
    }

    Ok(quote! {
        #(#asserts)*
        impl #impl_g ::coded_error::CodedError for #name #ty_g #where_g {
            fn code(&self) -> &'static str { match self { #(#code_arms,)* } }
            fn class(&self) -> ::coded_error::ErrorClass { match self { #(#class_arms,)* } }
            fn overrides(&self) -> ::coded_error::Overrides { match self { #(#override_arms,)* } }
        }
        impl #impl_g #name #ty_g #where_g {
            pub const ALL_CODES: &'static [&'static str] = &[#(#all_codes),*];
        }
    })
}

fn parse_namespace(input: &DeriveInput) -> syn::Result<LitStr> {
    let mut ns = None;
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("coded")) {
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("namespace") {
                ns = Some(m.value()?.parse::<LitStr>()?);
                Ok(())
            } else {
                Err(m.error("expected namespace = \"...\""))
            }
        })?;
    }
    ns.ok_or_else(|| syn::Error::new(input.ident.span(), "missing #[coded(namespace = \"...\")]"))
}

fn parse_variant(v: &syn::Variant) -> syn::Result<VariantSpec> {
    let attrs: Vec<_> = v.attrs.iter().filter(|a| a.path().is_ident("coded")).collect();
    if attrs.is_empty() {
        return Err(syn::Error::new(v.ident.span(), "every variant needs #[coded(...)]"));
    }
    let (mut code, mut grpc, mut http, mut mcp) = (None, None, None, None);
    let mut classes = Vec::new();
    for attr in attrs {
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("code") {
                if code.is_some() { return Err(m.error("code given twice")); }
                code = Some(m.value()?.parse::<LitStr>()?);
            } else if m.path.is_ident("transparent") {
                classes.push(ClassSource::Transparent);
            } else if m.path.is_ident("class_with") {
                classes.push(ClassSource::With(m.value()?.parse::<Path>()?));
            } else if m.path.is_ident("class") {
                let expr: Expr = m.value()?.parse()?;
                classes.push(parse_class_expr(&expr)?);
            } else if m.path.is_ident("grpc") {
                let g = m.value()?.parse::<Ident>()?;
                if g == "Ok" { return Err(syn::Error::new(g.span(), "grpc = Ok is not an error status")); }
                grpc = Some(g);
            } else if m.path.is_ident("http") {
                http = Some(m.value()?.parse::<LitInt>()?);
            } else if m.path.is_ident("mcp") {
                mcp = Some(m.value()?.parse::<Ident>()?);
            } else {
                return Err(m.error("unknown coded attribute"));
            }
            Ok(())
        })?;
    }
    if classes.len() != 1 {
        return Err(syn::Error::new(v.ident.span(), "exactly one class source"));
    }
    let class = classes.pop().unwrap();
    match (&class, &code) {
        (ClassSource::Transparent, Some(c)) => return Err(syn::Error::new(c.span(), "transparent variants take the inner code")),
        (ClassSource::Transparent, None) => {
            if v.fields.len() != 1 { return Err(syn::Error::new(v.ident.span(), "transparent needs exactly one field")); }
        }
        (_, None) => return Err(syn::Error::new(v.ident.span(), "missing code = \"...\"")),
        _ => {}
    }
    Ok(VariantSpec { ident: v.ident.clone(), fields: v.fields.clone(), code, class, grpc, http, mcp })
}

fn parse_class_expr(expr: &Expr) -> syn::Result<ClassSource> {
    match expr {
        Expr::Path(p) if p.path.get_ident().is_some() => {
            let id = p.path.get_ident().unwrap().clone();
            match id.to_string().as_str() {
                "Logical" | "Infrastructure" | "Transient" | "Unknown" => Ok(ClassSource::Fixed(id)),
                _ => Err(syn::Error::new(id.span(), "class must be Logical, Infrastructure, Transient, Unknown or from(field)")),
            }
        }
        Expr::Call(call) if matches!(&*call.func, Expr::Path(p) if p.path.is_ident("from")) && call.args.len() == 1 => {
            match &call.args[0] {
                Expr::Path(p) if p.path.get_ident().is_some() => Ok(ClassSource::FromField(FieldRef::Named(p.path.get_ident().unwrap().clone()))),
                Expr::Lit(ExprLit { lit: Lit::Int(i), .. }) => Ok(ClassSource::FromField(FieldRef::Index(i.base10_parse()?))),
                other => Err(syn::Error::new(other.span(), "from(...) takes a field name or tuple index")),
            }
        }
        other => Err(syn::Error::new(other.span(), "class must be Logical, Infrastructure, Transient, Unknown or from(field)")),
    }
}

fn overrides_tokens(s: &VariantSpec) -> TokenStream {
    let (grpc, http, mcp) = override_parts(s);
    quote! { ::coded_error::Overrides { grpc: #grpc, http: #http, mcp: #mcp } }
}

fn override_parts(s: &VariantSpec) -> (TokenStream, TokenStream, TokenStream) {
    let grpc = match &s.grpc { Some(g) => quote! { ::core::option::Option::Some(::coded_error::GrpcCode::#g) }, None => quote! { ::core::option::Option::None } };
    let http = match &s.http { Some(h) => quote! { ::core::option::Option::Some(#h) }, None => quote! { ::core::option::Option::None } };
    let mcp = match &s.mcp { Some(m) => quote! { ::core::option::Option::Some(::coded_error::McpCode::#m) }, None => quote! { ::core::option::Option::None } };
    (grpc, http, mcp)
}

fn wildcard_pattern(v: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(_) => quote! { Self::#v { .. } },
        Fields::Unnamed(_) => quote! { Self::#v(..) },
        Fields::Unit => quote! { Self::#v },
    }
}

fn single_field_pattern(v: &Ident, fields: &Fields, bind: &Ident) -> syn::Result<TokenStream> {
    match fields {
        Fields::Named(n) => {
            let f = n.named.first().unwrap().ident.as_ref().unwrap();
            Ok(quote! { Self::#v { #f: #bind } })
        }
        Fields::Unnamed(_) => Ok(quote! { Self::#v(#bind) }),
        Fields::Unit => Err(syn::Error::new(v.span(), "transparent needs exactly one field")),
    }
}

fn field_pattern(v: &Ident, fields: &Fields, f: &FieldRef, bind: &Ident) -> syn::Result<TokenStream> {
    match (fields, f) {
        (Fields::Named(n), FieldRef::Named(name)) => {
            if !n.named.iter().any(|x| x.ident.as_ref() == Some(name)) { return Err(syn::Error::new(name.span(), "no such field")); }
            Ok(quote! { Self::#v { #name: #bind, .. } })
        }
        (Fields::Unnamed(u), FieldRef::Index(i)) => {
            if *i >= u.unnamed.len() { return Err(syn::Error::new(v.span(), "tuple index out of range")); }
            let pats = (0..u.unnamed.len()).map(|k| if k == *i { quote! { #bind } } else { quote! { _ } });
            Ok(quote! { Self::#v(#(#pats),*) })
        }
        _ => Err(syn::Error::new(v.span(), "from(...) must name a field of this variant")),
    }
}
