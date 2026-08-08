//! Unit tests for the `archguard-extender` contract.
//!
//! Run with `cargo test --features testutils` (the Soroban SDK's test
//! helpers live behind the `testutils` feature).

use super::*;
use soroban_sdk::{
    symbol_short, testutils::{Address as _, Events}, token::{StellarAssetClient, TokenClient},
    vec, xdr, Address, Env, IntoVal, Map, Symbol, TryFromVal, TryIntoVal, Val, Vec,
};

/// Deploys the extender, initializes it with a fresh operator and the
/// native XLM SAC contract (all auth mocked), and returns the pieces tests
/// need along with the native asset address (tests build their own
/// `TokenClient` from it).
fn setup() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ExtenderContract, ());
    let operator = Address::generate(&env);
    let org = Address::generate(&env);
    let native = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();
    let client = ExtenderContractClient::new(&env, &contract_id);
    client.init(&operator, &native);
    (env, contract_id, org, operator, native)
}

/// Builds a token client for the native asset.
fn native_token<'a>(env: &'a Env, native: &Address) -> TokenClient<'a> {
    TokenClient::new(env, native)
}

/// Mints XLM to `to` via the native asset's SAC admin client (all auth is
/// mocked in `setup`).
fn fund(env: &Env, native: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, native).mint(to, &amount);
}

/// Returns `(topics, data)` of the event emitted by `contract` at index
/// `idx` within the last invocation.
fn emitted_event(env: &Env, contract: &Address, idx: usize) -> (Vec<Val>, Val) {
    let events = env.events().all().filter_by_contract(contract);
    let xdr_event = &events.events()[idx];
    let xdr::ContractEventBody::V0(v0) = &xdr_event.body;
    let mut topics: Vec<Val> = Vec::new(env);
    for t in v0.topics.iter() {
        topics.push_back(t.try_into_val(env).unwrap());
    }
    let data: Val = v0.data.try_into_val(env).unwrap();
    (topics, data)
}

/// The fixed event topic (struct name in snake_case).
fn event_name(env: &Env, name: &str) -> Val {
    Symbol::new(env, name).into_val(env)
}

#[test]
fn test_init() {
    let env = Env::default();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);

    let operator = Address::generate(&env);
    let native = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();
    client.init(&operator, &native);

    // init emits no event and leaves an unfunded org at balance 0.
    assert_eq!(env.events().all().events().len(), 0);
    assert_eq!(client.get_balance(&Address::generate(&env)), 0);
}

#[test]
#[should_panic]
fn test_init_twice_panics() {
    let env = Env::default();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);

    let native = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();
    client.init(&Address::generate(&env), &native);
    client.init(&Address::generate(&env), &native);
}

#[test]
fn test_deposit() {
    let (env, contract_id, org, _operator, native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    let token = native_token(&env, &native);
    fund(&env, &native, &org, 1_000_000);

    client.deposit(&org, &250_000);

    // deposited event with the org as topic and amount/balance as data.
    let (topics, data) = emitted_event(&env, &contract_id, 0);
    assert_eq!(
        topics,
        vec![&env, event_name(&env, "deposited"), org.clone().into_val(&env)]
    );
    let data_map: Map<Symbol, i128> = Map::try_from_val(&env, &data).unwrap();
    assert_eq!(data_map.get(symbol_short!("amount")).unwrap(), 250_000);
    assert_eq!(data_map.get(symbol_short!("balance")).unwrap(), 250_000);

    // Reads come after the event assertions (events are per-invocation).
    assert_eq!(client.get_balance(&org), 250_000);
    assert_eq!(token.balance(&contract_id), 250_000);
}

#[test]
fn test_deposit_invalid_amount() {
    let (env, contract_id, org, _operator, _native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    assert_eq!(client.try_deposit(&org, &0), Err(Ok(Error::InvalidAmount)));
    assert_eq!(client.try_deposit(&org, &-5), Err(Ok(Error::InvalidAmount)));
    assert_eq!(client.get_balance(&org), 0);
}

#[test]
fn test_deposit_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);
    // No init: no operator configured -> NotOperator.
    assert_eq!(
        client.try_deposit(&Address::generate(&env), &100),
        Err(Ok(Error::NotOperator))
    );
}

#[test]
#[should_panic]
fn test_deposit_requires_auth() {
    let env = Env::default();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);
    client.init(
        &Address::generate(&env),
        &env.register_stellar_asset_contract_v2(Address::generate(&env)).address(),
    );
    // No mock_all_auths: require_auth on `org` must fail.
    client.deposit(&Address::generate(&env), &100);
}

#[test]
fn test_withdraw() {
    let (env, contract_id, org, _operator, native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    let token = native_token(&env, &native);
    fund(&env, &native, &org, 1_000_000);
    client.deposit(&org, &500_000);
    client.withdraw(&org, &200_000);

    // withdrawn event with the org as topic.
    let (topics, data) = emitted_event(&env, &contract_id, 0);
    assert_eq!(
        topics,
        vec![&env, event_name(&env, "withdrawn"), org.clone().into_val(&env)]
    );
    let data_map: Map<Symbol, i128> = Map::try_from_val(&env, &data).unwrap();
    assert_eq!(data_map.get(symbol_short!("amount")).unwrap(), 200_000);
    assert_eq!(data_map.get(symbol_short!("balance")).unwrap(), 300_000);

    // Reads come after the event assertions (events are per-invocation).
    assert_eq!(client.get_balance(&org), 300_000);
    assert_eq!(token.balance(&contract_id), 300_000);
}

#[test]
fn test_withdraw_insufficient_balance() {
    let (env, contract_id, org, _operator, native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    fund(&env, &native, &org, 1_000_000);
    client.deposit(&org, &100_000);

    assert_eq!(
        client.try_withdraw(&org, &100_001),
        Err(Ok(Error::InsufficientBalance))
    );
    // Balance is unchanged and an unfunded org cannot withdraw at all.
    assert_eq!(client.get_balance(&org), 100_000);
    assert_eq!(
        client.try_withdraw(&Address::generate(&env), &1),
        Err(Ok(Error::InsufficientBalance))
    );
}

#[test]
fn test_withdraw_invalid_amount() {
    let (env, contract_id, org, _operator, _native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    assert_eq!(client.try_withdraw(&org, &0), Err(Ok(Error::InvalidAmount)));
    assert_eq!(client.try_withdraw(&org, &-1), Err(Ok(Error::InvalidAmount)));
}

#[test]
#[should_panic]
fn test_withdraw_requires_auth() {
    let env = Env::default();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);
    client.init(
        &Address::generate(&env),
        &env.register_stellar_asset_contract_v2(Address::generate(&env)).address(),
    );
    client.withdraw(&Address::generate(&env), &100);
}

#[test]
fn test_record_extension_cost() {
    let (env, contract_id, org, _operator, native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    fund(&env, &native, &org, 1_000_000);
    client.deposit(&org, &500_000);

    client.record_extension_cost(&org, &75_000);

    // extension_charged event carries cost and resulting balance.
    let (topics, data) = emitted_event(&env, &contract_id, 0);
    assert_eq!(
        topics,
        vec![&env, event_name(&env, "extension_charged"), org.clone().into_val(&env)]
    );
    let data_map: Map<Symbol, i128> = Map::try_from_val(&env, &data).unwrap();
    assert_eq!(data_map.get(symbol_short!("cost")).unwrap(), 75_000);
    assert_eq!(data_map.get(symbol_short!("balance")).unwrap(), 425_000);

    assert_eq!(client.get_balance(&org), 425_000);
}

#[test]
fn test_record_extension_cost_insufficient_balance() {
    let (env, contract_id, org, _operator, native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    fund(&env, &native, &org, 1_000_000);
    client.deposit(&org, &100_000);

    // Underfunded: no revert, balance unchanged, insufficient_balance emitted.
    client.record_extension_cost(&org, &150_000);

    let (topics, data) = emitted_event(&env, &contract_id, 0);
    assert_eq!(
        topics,
        vec![&env, event_name(&env, "insufficient_balance"), org.clone().into_val(&env)]
    );
    let data_map: Map<Symbol, i128> = Map::try_from_val(&env, &data).unwrap();
    assert_eq!(data_map.get(symbol_short!("cost")).unwrap(), 150_000);
    assert_eq!(data_map.get(symbol_short!("balance")).unwrap(), 100_000);

    // The balance was left untouched.
    assert_eq!(client.get_balance(&org), 100_000);
}

#[test]
fn test_record_extension_cost_invalid_amount() {
    let (env, contract_id, org, _operator, native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    fund(&env, &native, &org, 1_000_000);
    client.deposit(&org, &100_000);
    assert_eq!(
        client.try_record_extension_cost(&org, &-1),
        Err(Ok(Error::InvalidAmount))
    );
}

#[test]
fn test_record_extension_cost_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);
    assert_eq!(
        client.try_record_extension_cost(&Address::generate(&env), &100),
        Err(Ok(Error::NotOperator))
    );
}

#[test]
#[should_panic]
fn test_record_extension_cost_requires_operator_auth() {
    // The keeper (operator) authorizes this call — NOT the org. Without any
    // mocked auth, the operator's require_auth must fail even though the
    // caller presents an org address.
    let env = Env::default();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);
    client.init(
        &Address::generate(&env),
        &env.register_stellar_asset_contract_v2(Address::generate(&env)).address(),
    );
    client.record_extension_cost(&Address::generate(&env), &10_000);
}

#[test]
fn test_get_balance_defaults_to_zero() {
    let (env, contract_id, _org, _operator, _native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);
    // Read-only: never funded, never touched -> 0. No auth needed.
    assert_eq!(client.get_balance(&Address::generate(&env)), 0);
}

#[test]
fn test_set_operator() {
    let (env, contract_id, _org, _operator, _native) = setup();
    let client = ExtenderContractClient::new(&env, &contract_id);

    let before: Operator = env.as_contract(&contract_id, || {
        env.storage().instance().get(&DataKey::Operator).unwrap()
    });
    let new_operator = Address::generate(&env);
    client.set_operator(&new_operator);

    // The operator rotated; the injected native asset address is preserved.
    let after: Operator = env.as_contract(&contract_id, || {
        env.storage().instance().get(&DataKey::Operator).unwrap()
    });
    assert_eq!(after.operator, new_operator);
    assert_eq!(after.native_asset, before.native_asset);
}

#[test]
fn test_set_operator_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);
    // No init: no operator configured -> NotOperator.
    assert_eq!(
        client.try_set_operator(&Address::generate(&env)),
        Err(Ok(Error::NotOperator))
    );
}

#[test]
#[should_panic]
fn test_set_operator_requires_current_operator_auth() {
    let env = Env::default();
    let contract_id = env.register(ExtenderContract, ());
    let client = ExtenderContractClient::new(&env, &contract_id);
    client.init(
        &Address::generate(&env),
        &env.register_stellar_asset_contract_v2(Address::generate(&env)).address(),
    );
    // No mock_all_auths: the current operator must sign the rotation.
    client.set_operator(&Address::generate(&env));
}
