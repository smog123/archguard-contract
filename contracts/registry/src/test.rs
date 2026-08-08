//! Unit tests for the `archguard-registry` contract.
//!
//! Run with `cargo test --features testutils` (the Soroban SDK's test
//! helpers live behind the `testutils` feature).

use super::*;
use soroban_sdk::{
    symbol_short, testutils::{Address as _, Events}, vec, xdr, Address, BytesN, Env, IntoVal,
    Map, Symbol, TryFromVal, TryIntoVal, Val,
};

/// Deploys the registry, initializes it, registers `org` (with all auth
/// mocked), and returns the pieces tests need.
fn setup() -> (Env, Address, Address, Address, BytesN<32>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RegistryContract, ());
    let org = Address::generate(&env);
    let admin = Address::generate(&env);
    let webhook = BytesN::from_array(&env, &[7u8; 32]);
    let client = RegistryContractClient::new(&env, &contract_id);
    client.init();
    client.register_org(&org, &admin, &webhook);
    (env, contract_id, org, admin, webhook)
}

/// Returns `(topics, data)` of the event emitted by `contract` at index
/// `idx` (0-based, in publish order across all invocations so far).
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
    let contract_id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(&env, &contract_id);
    // init is callable without auth and emits nothing.
    client.init();
    assert_eq!(env.events().all().events().len(), 0);
}

#[test]
#[should_panic]
fn test_init_twice_panics() {
    let env = Env::default();
    let contract_id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(&env, &contract_id);
    client.init();
    client.init();
}

#[test]
fn test_register_org() {
    let (env, contract_id, org, admin, webhook) = setup();

    let (topics, data) = emitted_event(&env, &contract_id, 0);
    assert_eq!(
        topics,
        vec![
            &env,
            event_name(&env, "org_registered"),
            org.clone().into_val(&env),
        ]
    );
    let config_map: Map<Symbol, OrgConfig> = Map::try_from_val(&env, &data).unwrap();
    assert_eq!(
        config_map.get(symbol_short!("config")).unwrap(),
        OrgConfig {
            admin: admin.clone(),
            notify_webhook: webhook.clone(),
            active: true,
        }
    );
}

#[test]
#[should_panic]
fn test_register_org_twice_panics() {
    let (env, contract_id, org, _admin, _webhook) = setup();
    let client = RegistryContractClient::new(&env, &contract_id);
    // Duplicate registration is a caller error and must panic.
    client.register_org(&org, &Address::generate(&env), &BytesN::from_array(&env, &[9u8; 32]));
}

#[test]
#[should_panic]
fn test_register_org_requires_auth() {
    let env = Env::default();
    let contract_id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(&env, &contract_id);
    client.init();
    // No mock_all_auths: require_auth on `org` must fail.
    client.register_org(
        &Address::generate(&env),
        &Address::generate(&env),
        &BytesN::from_array(&env, &[1u8; 32]),
    );
}
