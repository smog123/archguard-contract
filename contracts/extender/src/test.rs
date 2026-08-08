//! Unit tests for the `archguard-extender` contract.
//!
//! Run with `cargo test --features testutils` (the Soroban SDK's test
//! helpers live behind the `testutils` feature).

use super::*;
use soroban_sdk::{
    symbol_short, testutils::{Address as _, Events}, token::TokenClient, vec, xdr, Address, Env,
    IntoVal, Map, Symbol, TryFromVal, TryIntoVal, Val, Vec,
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
