#![no_std]

mod errors;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Val, Vec};
use types::{DataKey, Durability, OrgConfig, WatchedEntry};

/// Re-extend a stored entry once its remaining TTL drops below this many
/// ledgers (~1 day at 5s/ledger).
const TTL_THRESHOLD: u32 = 17_280;
/// Extend stored entries out to this many ledgers (~30 days at 5s/ledger).
const TTL_EXTEND_TO: u32 = 518_400;

/// Extends the TTL of the contract instance entry after every
/// state-changing call, per the Archguard storage policy.
fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
}

/// Extends the TTL of a persistent entry immediately after writing it, per
/// the Archguard storage policy.
fn extend_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

/// `archguard-registry` — the on-chain watch-list for Archguard.
///
/// Teams register orgs and add "watched entries" (a contract + storage key
/// pair) whose TTL the off-chain keeper monitors and auto-extends. All
/// coordination with `archguard-extender` happens off-chain; the two
/// contracts never call each other on-chain.
#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
    /// Initializes the registry contract.
    ///
    /// Seeds the watched-entry id counter and guards against
    /// re-initialization: calling `init` twice panics. Idempotent callers
    /// (e.g. the deployer tooling) should treat a panic here as "already
    /// initialized" and proceed.
    ///
    /// # Auth
    ///
    /// No auth required — any caller may initialize, but only the first
    /// call succeeds.
    pub fn init(env: Env) {
        if env.storage().instance().has(&DataKey::NextEntryId) {
            panic!("registry already initialized");
        }
        env.storage().instance().set(&DataKey::NextEntryId, &0u64);
        extend_instance_ttl(&env);
    }
}
