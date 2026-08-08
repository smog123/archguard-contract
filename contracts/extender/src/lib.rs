#![no_std]

mod errors;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env};
use types::{DataKey, InsufficientBalance, Operator};

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

    /// Withdraws native XLM from the extender back to `org`, debiting the
    /// org's prepaid balance, in stroops.
    ///
    /// # Auth
    ///
    /// Requires auth from `org`.
    ///
    /// # Errors
    ///
    /// - [`Error::NotOperator`] if the contract is not initialized.
    /// - [`Error::InvalidAmount`] if `amount <= 0`.
    /// - [`Error::InsufficientBalance`] if `amount` exceeds the org's
    ///   prepaid balance.
    pub fn withdraw(env: Env, org: Address, amount: i128) -> Result<(), Error> {
        org.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let balance = get_balance(&env, &org);
        if amount > balance {
            return Err(Error::InsufficientBalance);
        }
        let op = load_operator(&env)?;

        let token = TokenClient::new(&env, &op.native_asset);
        token.transfer(&env.current_contract_address(), &org, &amount);

        let new_balance = balance - amount;
        env.storage()
            .instance()
            .set(&DataKey::OrgBalance(org.clone()), &new_balance);
        extend_instance_ttl(&env);

        Ok(())
    }

    /// Records a keeper-charged extension cost against the org's prepaid
    /// balance, in stroops.
    ///
    /// The actual TTL extension happens off-chain via `ExtendFootprintTTLOp`
    /// (that ledger operation must be the sole operation of its
    /// transaction, so it cannot be wrapped inside a contract-to-contract
    /// call); the keeper calls this afterwards to debit the org.
    ///
    /// # Auth
    ///
    /// Requires auth from the **operator** (the keeper), not the org — the
    /// keeper is the one charging the org for extension work it performed.
    ///
    /// # Errors
    ///
    /// - [`Error::NotOperator`] if the contract is not initialized.
    /// - [`Error::InvalidAmount`] if `cost < 0`.
    ///
    /// # Underfunded orgs — deliberate design choice
    ///
    /// When the org's balance is less than the cost, the transaction does
    /// **not** revert: the contract emits `insufficient_balance` and
    /// returns `Ok(())` with the balance unchanged, so the keeper learns
    /// the org is underfunded without a destructive rollback. (Returning
    /// `Err(Error::InsufficientBalance)` would revert the whole call and
    /// hide the very information the keeper needs to alert the org.)
    pub fn record_extension_cost(env: Env, org: Address, cost: i128) -> Result<(), Error> {
        let op = load_operator(&env)?;
        op.operator.require_auth();

        if cost < 0 {
            return Err(Error::InvalidAmount);
        }

        let balance = get_balance(&env, &org);
        if balance < cost {
            InsufficientBalance { org: org.clone(), cost, balance }.publish(&env);
            return Ok(());
        }

        let new_balance = balance - cost;
        env.storage()
            .instance()
            .set(&DataKey::OrgBalance(org.clone()), &new_balance);
        extend_instance_ttl(&env);

        Ok(())
    }

    /// Returns the org's prepaid balance in stroops (0 when never funded).
    ///
    /// Read-only: does **not** require auth.
    pub fn get_balance(env: Env, org: Address) -> i128 {
        get_balance(&env, &org)
    }

    /// Replaces the operator (keeper) address. The injected native asset
    /// address is preserved.
    ///
    /// # Auth
    ///
    /// Requires auth from the **current** operator.
    ///
    /// # Errors
    ///
    /// [`Error::NotOperator`] if the contract is not initialized (no
    /// operator is configured yet).
    pub fn set_operator(env: Env, new_operator: Address) -> Result<(), Error> {
        let op = load_operator(&env)?;
        op.operator.require_auth();

        env.storage().instance().set(
            &DataKey::Operator,
            &Operator {
                operator: new_operator,
                native_asset: op.native_asset,
            },
        );
        extend_instance_ttl(&env);
        Ok(())
    }
}
