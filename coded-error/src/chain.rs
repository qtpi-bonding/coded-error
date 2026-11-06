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
            if let Some(codes) = remote_codes {
                if codes.is_empty() {
                    out.push(link);
                } else {
                    let own_code = link.code;
                    let mut message_assigned = false;
                    let first_expanded = out.len();
                    out.extend(codes.into_iter().map(|code| {
                        let is_message = !message_assigned && own_code == Some(code);
                        if is_message {
                            message_assigned = true;
                        }
                        ChainLink {
                            code: Some(code),
                            class: None,
                            type_name: None,
                            origin: Origin::External,
                            message: if is_message {
                                link.message.clone()
                            } else {
                                String::new()
                            },
                        }
                    }));
                    if !message_assigned {
                        if let Some(first) = out.get_mut(first_expanded) {
                            // The first expanded link owns the remote text
                            // when the peer omitted the status code.
                            first.message = link.message;
                        }
                    }
                }
            } else {
                out.push(link);
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
