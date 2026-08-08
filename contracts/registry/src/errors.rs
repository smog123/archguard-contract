//! Errors returned by the `archguard-registry` contract.

use soroban_sdk::contracterror;

/// Errors returned by the `archguard-registry` contract.
#[contracterror]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Error {
    /// The org has not been registered yet.
    OrgNotFound = 1,
    /// No watched entry exists with the given id.
    EntryNotFound = 2,
    /// The org does not own the watched entry it is trying to modify.
    NotEntryOwner = 3,
    /// The org has been deactivated and cannot make changes.
    OrgInactive = 4,
    /// `extend_threshold_ledgers` is not strictly below `extend_to_ledgers`.
    InvalidThreshold = 5,
}
