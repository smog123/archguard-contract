//! Shared types for the `archguard-extender` contract.

use soroban_sdk::{contractevent, contracttype, Address};

/// Storage keys used by the extender contract.
///
/// Both keys live in **instance** storage — per-org balances and the
/// operator config are small and always needed together. `Temporary`
/// storage is never used by this contract.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataKey {
    /// Prepaid XLM balance (stroops) held in custody for an org.
    OrgBalance(Address),
    /// The operator (keeper) configuration.
    Operator,
}

/// Operator (keeper) configuration, stored under [`DataKey::Operator`].
///
/// The value is a small struct rather than a bare `Address` because the
/// native XLM asset contract (SAC) address is network-dependent and must be
/// injected at init time (never hardcoded). Keeping the `DataKey` enum
/// exactly as specified, the asset address rides along with the operator.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operator {
    /// The keeper address authorized to charge extension costs.
    pub operator: Address,
    /// The native XLM SAC contract address for the network in use
    /// (testnet vs mainnet).
    pub native_asset: Address,
}

/// Emitted when an org deposits XLM. Topics: `["deposited", org]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct Deposited {
    #[topic]
    pub org: Address,
    pub amount: i128,
    pub balance: i128,
}

/// Emitted when an org withdraws XLM. Topics: `["withdrawn", org]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct Withdrawn {
    #[topic]
    pub org: Address,
    pub amount: i128,
    pub balance: i128,
}

/// Emitted when the keeper charges an extension cost.
/// Topics: `["extension_charged", org]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct ExtensionCharged {
    #[topic]
    pub org: Address,
    pub cost: i128,
    pub balance: i128,
}

/// Emitted when a keeper records a cost the org's balance cannot cover.
/// Topics: `["insufficient_balance", org]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct InsufficientBalance {
    #[topic]
    pub org: Address,
    pub cost: i128,
    pub balance: i128,
}
