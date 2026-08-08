//! Shared types for the `archguard-registry` contract.

use soroban_sdk::{contracttype, Address, BytesN, Val};

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
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchedEntry {
    /// Globally unique entry id, assigned by [`super::RegistryContract`].
    pub id: u64,
    /// Org that owns this entry.
    pub org: Address,
    /// Contract whose storage this entry refers to.
    pub contract_id: Address,
    /// Durability category of the watched storage key.
    pub durability: Durability,
    /// The storage key being watched, if any (absent when the whole
    /// contract instance is watched).
    pub key: Option<Val>,
    /// Re-extend when the remaining TTL (ledgers) drops below this value.
    pub extend_threshold_ledgers: u32,
    /// Extend the entry's TTL out to this value (ledgers).
    pub extend_to_ledgers: u32,
    /// Whether the keeper is authorized to auto-extend without asking.
    pub auto_extend: bool,
    /// Ledger timestamp when the entry was added.
    pub created_at: u64,
}
