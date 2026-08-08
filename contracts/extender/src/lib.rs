#![no_std]

mod errors;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env};
use types::{DataKey, Operator};

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

/// Loads the operator config.
///
/// Returns [`Error::NotOperator`] when the contract has not been
/// initialized yet — i.e. no operator is configured. (`NotOperator` is the
/// closest variant in the spec's fixed error list; it doubles as the
/// "not initialized" signal.)
fn load_operator(env: &Env) -> Result<Operator, Error> {
    env.storage()
        .instance()
        .get::<DataKey, Operator>(&DataKey::Operator)
        .ok_or(Error::NotOperator)
}

/// Returns the org's prepaid balance, defaulting to 0 when unset.
fn get_balance(env: &Env, org: &Address) -> i128 {
    env.storage()
        .instance()
        .get::<DataKey, i128>(&DataKey::OrgBalance(org.clone()))
        .unwrap_or(0)
}

/// `archguard-extender` — fund custody and extension accounting for
/// Archguard.
///
/// Holds per-org prepaid XLM balances and records the extension costs the
/// off-chain keeper charges against them. Fund custody is isolated in this
/// contract so the sensitive money logic is auditable separately from the
/// mostly-public-read registry. All coordination with
/// `archguard-registry` happens off-chain; the two contracts never call
/// each other on-chain.
#[contract]
pub struct ExtenderContract;

#[contractimpl]
impl ExtenderContract {
    /// Initializes the extender with the keeper (operator) address and the
    /// native XLM asset contract (SAC) address for the network in use.
    ///
    /// The asset address is deliberately injected rather than hardcoded:
    /// it differs between networks. Only callable once — a second `init`
    /// panics. Idempotent callers (e.g. deployer tooling) should treat a
    /// panic here as "already initialized".
    ///
    /// # Auth
    ///
    /// No auth required — any caller may initialize, but only the first
    /// call succeeds.
    pub fn init(env: Env, operator: Address, native_asset: Address) {
        if env.storage().instance().has(&DataKey::Operator) {
            panic!("contract already initialized");
        }
        env.storage()
            .instance()
            .set(&DataKey::Operator, &Operator { operator, native_asset });
        extend_instance_ttl(&env);
    }

    /// Deposits native XLM from `org` into the extender's custody and
    /// credits `org`'s prepaid balance, in stroops.
    ///
    /// The transfer is executed against the native XLM SAC contract whose
    /// address was injected at init time.
    ///
    /// # Auth
    ///
    /// Requires auth from `org` (the depositor).
    ///
    /// # Errors
    ///
    /// - [`Error::NotOperator`] if the contract is not initialized.
    /// - [`Error::InvalidAmount`] if `amount <= 0`.
    pub fn deposit(env: Env, org: Address, amount: i128) -> Result<(), Error> {
        org.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let op = load_operator(&env)?;

        let token = TokenClient::new(&env, &op.native_asset);
        token.transfer(&org, &env.current_contract_address(), &amount);

        let new_balance = get_balance(&env, &org) + amount;
        env.storage()
            .instance()
            .set(&DataKey::OrgBalance(org.clone()), &new_balance);
        extend_instance_ttl(&env);

        Ok(())
    }
}
