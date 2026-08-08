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

    /// Registers a new org with the registry.
    ///
    /// Stores the org's [`OrgConfig`]. Only the SHA-256 hash of the
    /// notification webhook is stored on-chain — never the raw URL
    /// (storage-cost rationale); the off-chain keeper resolves the hash.
    ///
    /// # Auth
    ///
    /// Requires auth from `org` (the org self-registers).
    ///
    /// # Errors
    ///
    /// Panics if the org is already registered.
    pub fn register_org(env: Env, org: Address, admin: Address, notify_webhook: BytesN<32>) {
        org.require_auth();
        if env.storage().instance().has(&DataKey::Org(org.clone())) {
            panic!("org already registered");
        }
        let config = OrgConfig {
            admin,
            notify_webhook,
            active: true,
        };
        env.storage().instance().set(&DataKey::Org(org.clone()), &config);
        extend_instance_ttl(&env);
    }

    /// Adds a new watched entry for an org and returns its id.
    ///
    /// Ids are assigned from the `NextEntryId` counter, which starts at 0
    /// and is incremented after every successful call, so the first entry
    /// receives id 1. The entry record and the org's id list are written to
    /// persistent storage and their TTL is extended immediately, per the
    /// Archguard storage policy.
    ///
    /// # Auth
    ///
    /// Requires auth from `org` (the org that will own the entry).
    ///
    /// # Errors
    ///
    /// - [`Error::OrgNotFound`] if the org is not registered.
    /// - [`Error::OrgInactive`] if the org has been deactivated.
    /// - [`Error::InvalidThreshold`] if `extend_threshold_ledgers` is not
    ///   strictly below `extend_to_ledgers`.
    pub fn add_watched_entry(
        env: Env,
        org: Address,
        contract_id: Address,
        durability: Durability,
        key: Option<Val>,
        extend_threshold_ledgers: u32,
        extend_to_ledgers: u32,
        auto_extend: bool,
    ) -> Result<u64, Error> {
        org.require_auth();

        let config = env
            .storage()
            .instance()
            .get::<DataKey, OrgConfig>(&DataKey::Org(org.clone()))
            .ok_or(Error::OrgNotFound)?;
        if !config.active {
            return Err(Error::OrgInactive);
        }
        if extend_threshold_ledgers >= extend_to_ledgers {
            return Err(Error::InvalidThreshold);
        }

        let id: u64 =
            env.storage().instance().get(&DataKey::NextEntryId).unwrap_or(0) + 1;
        env.storage().instance().set(&DataKey::NextEntryId, &id);

        let entry = WatchedEntry {
            id,
            org: org.clone(),
            contract_id,
            durability,
            key,
            extend_threshold_ledgers,
            extend_to_ledgers,
            auto_extend,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&DataKey::WatchedEntry(id), &entry);

        let mut org_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OrgEntryIds(org.clone()))
            .unwrap_or(Vec::new(&env));
        org_ids.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::OrgEntryIds(org.clone()), &org_ids);

        extend_persistent_ttl(&env, &DataKey::WatchedEntry(id));
        extend_persistent_ttl(&env, &DataKey::OrgEntryIds(org.clone()));
        extend_instance_ttl(&env);

        Ok(id)
    }

    /// Removes a watched entry and drops its id from the org's entry list.
    ///
    /// # Auth
    ///
    /// Requires auth from `org` (the entry owner).
    ///
    /// # Errors
    ///
    /// - [`Error::OrgNotFound`] if the org is not registered.
    /// - [`Error::EntryNotFound`] if no entry exists with `entry_id`.
    /// - [`Error::NotEntryOwner`] if the entry belongs to a different org.
    pub fn remove_watched_entry(env: Env, org: Address, entry_id: u64) -> Result<(), Error> {
        org.require_auth();

        env.storage()
            .instance()
            .get::<DataKey, OrgConfig>(&DataKey::Org(org.clone()))
            .ok_or(Error::OrgNotFound)?;

        let entry = env
            .storage()
            .persistent()
            .get::<DataKey, WatchedEntry>(&DataKey::WatchedEntry(entry_id))
            .ok_or(Error::EntryNotFound)?;
        if entry.org != org {
            return Err(Error::NotEntryOwner);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::WatchedEntry(entry_id));

        let mut org_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OrgEntryIds(org.clone()))
            .unwrap_or(Vec::new(&env));
        if let Some(index) = org_ids.first_index_of(&entry_id) {
            org_ids.remove(index);
        }
        env.storage()
            .persistent()
            .set(&DataKey::OrgEntryIds(org.clone()), &org_ids);

        extend_persistent_ttl(&env, &DataKey::OrgEntryIds(org));
        extend_instance_ttl(&env);

        Ok(())
    }

    /// Updates the extension policy of an existing watched entry.
    ///
    /// Only the entry's owning org can update its policy.
    ///
    /// # Auth
    ///
    /// Requires auth from `org` (the entry owner).
    ///
    /// # Errors
    ///
    /// - [`Error::OrgNotFound`] if the org is not registered.
    /// - [`Error::EntryNotFound`] if no entry exists with `entry_id`.
    /// - [`Error::NotEntryOwner`] if the entry belongs to a different org.
    /// - [`Error::InvalidThreshold`] if `extend_threshold_ledgers` is not
    ///   strictly below `extend_to_ledgers`.
    pub fn update_entry_policy(
        env: Env,
        org: Address,
        entry_id: u64,
        extend_threshold_ledgers: u32,
        extend_to_ledgers: u32,
        auto_extend: bool,
    ) -> Result<(), Error> {
        org.require_auth();

        env.storage()
            .instance()
            .get::<DataKey, OrgConfig>(&DataKey::Org(org.clone()))
            .ok_or(Error::OrgNotFound)?;

        let mut entry = env
            .storage()
            .persistent()
            .get::<DataKey, WatchedEntry>(&DataKey::WatchedEntry(entry_id))
            .ok_or(Error::EntryNotFound)?;
        if entry.org != org {
            return Err(Error::NotEntryOwner);
        }
        if extend_threshold_ledgers >= extend_to_ledgers {
            return Err(Error::InvalidThreshold);
        }

        entry.extend_threshold_ledgers = extend_threshold_ledgers;
        entry.extend_to_ledgers = extend_to_ledgers;
        entry.auto_extend = auto_extend;

        env.storage()
            .persistent()
            .set(&DataKey::WatchedEntry(entry_id), &entry);
        extend_persistent_ttl(&env, &DataKey::WatchedEntry(entry_id));
        extend_instance_ttl(&env);

        Ok(())
    }
}
