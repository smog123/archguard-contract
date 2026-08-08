//! Errors returned by the `archguard-extender` contract.

use soroban_sdk::contracterror;

/// Errors returned by the `archguard-extender` contract.
#[contracterror]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Error {
    /// The org's prepaid balance cannot cover the requested amount.
    InsufficientBalance = 1,
    /// No operator is configured (contract not initialized) or the caller
    /// is not the operator.
    NotOperator = 2,
    /// The amount is not a positive, valid amount.
    InvalidAmount = 3,
}
