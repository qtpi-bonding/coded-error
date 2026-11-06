use crate::ErrorClass;
use std::error::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Own,
    External,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainLink {
    pub code: Option<&'static str>,
    pub class: Option<ErrorClass>,
    pub type_name: Option<&'static str>,
    pub origin: Origin,
    pub message: String,
}

#[cfg(feature = "anyhow")]
pub fn chain_anyhow(err: &anyhow::Error) -> Vec<ChainLink> {
    chain(&**err)
}

#[cfg(feature = "anyhow")]
fn coded_link(err: &(dyn Error + 'static)) -> Option<(ChainLink, Option<Vec<&'static str>>)> {
    let c = err.downcast_ref::<crate::Coded>()?;
    let link = c.chain_link();
    let remote = c.remote_chain().map(|codes| codes.to_vec());
    Some((link, remote))
}

#[cfg(not(feature = "anyhow"))]
fn coded_link(_err: &(dyn Error + 'static)) -> Option<(ChainLink, Option<Vec<&'static str>>)> {
    None
}

pub fn chain(e: &(dyn Error + 'static)) -> Vec<ChainLink> {
    let mut out = Vec::new();
    let mut cur: Option<&(dyn Error + 'static)> = Some(e);
    while let Some(err) = cur {
        if let Some((link, remote_codes)) = coded_link(err) {
            out.push(link);
            if let Some(codes) = remote_codes {
                out.extend(codes.into_iter().map(|code| ChainLink {
                    code: Some(code),
                    class: None,
                    type_name: None,
                    origin: Origin::External,
                    message: String::new(),
                }));
            }
        } else {
            out.push(external_link(err));
        }
        cur = err.source();
    }
    out
}

fn external_link(err: &(dyn Error + 'static)) -> ChainLink {
    ChainLink {
        code: None,
        class: None,
        type_name: None,
        origin: Origin::External,
        message: err.to_string(),
    }
}
