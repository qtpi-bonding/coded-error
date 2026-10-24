//! Exit codes for `Err` paths only. Codes a command returns on `Ok`
//! (archegraph's 2 and 3) never pass through here.
use crate::{CodedError, ErrorClass};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitTable {
    pub logical: u8,
    pub infrastructure: u8,
    pub transient: u8,
    pub unknown: u8,
}

impl ExitTable {
    /// sysexits: DATAERR 65, UNAVAILABLE 69, TEMPFAIL 75, SOFTWARE 70.
    pub const SYSEXITS: ExitTable = ExitTable {
        logical: 65,
        infrastructure: 69,
        transient: 75,
        unknown: 70,
    };

    /// One code for every error, for repos with a published contract.
    pub const fn all(code: u8) -> ExitTable {
        ExitTable {
            logical: code,
            infrastructure: code,
            transient: code,
            unknown: code,
        }
    }

    pub const fn code_for(&self, class: ErrorClass) -> u8 {
        match class {
            ErrorClass::Logical => self.logical,
            ErrorClass::Infrastructure => self.infrastructure,
            ErrorClass::Transient => self.transient,
            ErrorClass::Unknown => self.unknown,
        }
    }
}

pub fn exit_code<E: CodedError + ?Sized>(e: &E) -> u8 {
    ExitTable::SYSEXITS.code_for(e.class())
}

pub fn exit_code_with<E: CodedError + ?Sized>(e: &E, table: &ExitTable) -> u8 {
    table.code_for(e.class())
}

#[cfg(feature = "anyhow")]
pub fn exit_code_anyhow(err: &anyhow::Error) -> u8 {
    exit_code_anyhow_with(err, &ExitTable::SYSEXITS)
}

#[cfg(feature = "anyhow")]
pub fn exit_code_anyhow_with(err: &anyhow::Error, table: &ExitTable) -> u8 {
    table.code_for(crate::facts_anyhow(err).class)
}
