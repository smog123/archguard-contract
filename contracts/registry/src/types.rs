//! Shared types for the `archguard-registry` contract.

use soroban_sdk::{contractevent, contracttype, Address, Bytes, BytesN};

/// Storage keys used by the registry contract.
///
/// Storage placement follows the Archguard storage policy:
/// - [`DataKey::Org`] and [`DataKey::NextEntryId`] live in **instance**
///   storage — org configs are small, few, and always needed together.
/// - [`DataKey::WatchedEntry`] and [`DataKey::OrgEntryIds`] live in
///   **persistent** storage — they scale with usage and must not bloat the
///   contract instance entry.
///
/// `Temporary` storage is never used by this contract.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataKey {
    Org(Address),
    WatchedEntry(u64),
    OrgEntryIds(Address),
    NextEntryId,
}

/// The durability category of a watched entry's underlying storage.
///
/// Soroban SDK 27 removed the public `soroban_sdk::storage::Durability`
/// enum (storage is now accessed through the typed
/// `Instance`/`Persistent`/`Temporary` handles), so this contract defines
/// its own enum with the same variants.
///
/// Only [`Durability::Instance`] and [`Durability::Persistent`] are in
/// scope for Archguard; [`Durability::Temporary`] is excluded by design —
/// temporary data cannot be meaningfully TTL-watched.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Durability {
    Instance,
    Persistent,
    Temporary,
}

/// Configuration for a registered org.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrgConfig {
    /// Address authorized to manage this org's watched entries.
    pub admin: Address,
    /// SHA-256 hash of the org's notification webhook URL. Only the hash is
    /// stored on-chain (storage-cost rationale); the off-chain keeper
    /// resolves the hash to the raw URL.
    pub notify_webhook: BytesN<32>,
    /// Whether the org is currently active. Deactivated orgs cannot add
    /// watched entries.
    pub active: bool,
}

/// A single watched entry: one contract storage key the keeper watches and
/// auto-extends.
///
/// Note: this type deliberately does **not** derive `PartialEq`/`Eq` —
/// SDK 27's `Val` (used by the optional `key` field) implements neither.
#[contracttype]
#[derive(Clone, Debug)]
pub struct WatchedEntry {
    /// Globally unique entry id, assigned by [`super::RegistryContract`].
    pub id: u64,
    /// Org that owns this entry.
    pub org: Address,
    /// Contract whose storage this entry refers to.
    pub contract_id: Address,
    /// Durability category of the watched storage key.
    pub durability: Durability,
    /// The storage key being watched, as raw bytes, if any (absent when
    /// the whole contract instance is watched).
    ///
    /// Note: the original design used `Option<Val>`, but Soroban SDK 27's
    /// `#[contracttype]` derive requires fields to convert via `Into<ScVal>`
    /// (std), which `Val` does not implement — `Bytes` does, and a storage
    /// key is consumed by the off-chain keeper as raw bytes anyway.
    pub key: Option<Bytes>,
    /// Re-extend when the remaining TTL (ledgers) drops below this value.
    pub extend_threshold_ledgers: u32,
    /// Extend the entry's TTL out to this value (ledgers).
    pub extend_to_ledgers: u32,
    /// Whether the keeper is authorized to auto-extend without asking.
    pub auto_extend: bool,
    /// Ledger timestamp when the entry was added.
    pub created_at: u64,
}

/// Emitted when a new org registers. Topics: `["org_registered", org]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct OrgRegistered {
    #[topic]
    pub org: Address,
    pub config: OrgConfig,
}

/// Emitted when a watched entry is added. Topics: `["entry_added", id]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct EntryAdded {
    #[topic]
    pub entry_id: u64,
    pub entry: WatchedEntry,
}

/// Emitted when a watched entry is removed. Topics: `["entry_removed", id]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct EntryRemoved {
    #[topic]
    pub entry_id: u64,
}

/// Emitted when a watched entry's extension policy changes.
/// Topics: `["entry_policy_updated", id]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct EntryPolicyUpdated {
    #[topic]
    pub entry_id: u64,
    pub entry: WatchedEntry,
}

/// Emitted when an org is deactivated. Topics: `["org_deactivated", org]`.
#[contractevent]
#[derive(Clone, Debug)]
pub struct OrgDeactivated {
    #[topic]
    pub org: Address,
}
