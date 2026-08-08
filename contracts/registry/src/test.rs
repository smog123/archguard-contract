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

#[test]
fn test_add_watched_entry() {
    let (env, contract_id, org, _admin, _webhook) = setup();
    let watched = Address::generate(&env);
    let client = RegistryContractClient::new(&env, &contract_id);

    // First entry gets id 1.
    let id = client.add_watched_entry(
        &org, &watched, &Durability::Persistent, &None::<Bytes>, &1_000, &10_000, &true,
    );
    assert_eq!(id, 1);

    // entry_added event for the first entry: topics [entry_added, id].
    let (topics, data) = emitted_event(&env, &contract_id, 0);
    assert_eq!(
        topics,
        vec![&env, event_name(&env, "entry_added"), id.into_val(&env)]
    );
    let data_map: Map<Symbol, WatchedEntry> = Map::try_from_val(&env, &data).unwrap();
    let entry = data_map.get(symbol_short!("entry")).unwrap();
    assert_eq!(entry.id, 1);
    assert_eq!(entry.org, org);
    assert_eq!(entry.contract_id, watched);
    assert_eq!(entry.durability, Durability::Persistent);
    assert_eq!(entry.key, None);
    assert_eq!(entry.extend_threshold_ledgers, 1_000);
    assert_eq!(entry.extend_to_ledgers, 10_000);
    assert!(entry.auto_extend);
    assert_eq!(entry.created_at, env.ledger().timestamp());

    // Second entry gets id 2 (NextEntryId increments after each call) and
    // carries the raw key bytes.
    let key = Bytes::from_slice(&env, &[1, 2, 3]);
    let id2 = client.add_watched_entry(
        &org, &watched, &Durability::Instance, &Some(key.clone()), &5_000, &50_000, &false,
    );
    assert_eq!(id2, 2);
    let (topics2, _) = emitted_event(&env, &contract_id, 0);
    assert_eq!(
        topics2,
        vec![&env, event_name(&env, "entry_added"), id2.into_val(&env)]
    );

    // The second entry's stored fields round-trip through get_entry.
    let entry2 = client.get_entry(&id2);
    assert_eq!(entry2.key, Some(key));
    assert_eq!(entry2.durability, Durability::Instance);
    assert!(!entry2.auto_extend);
}

#[test]
fn test_add_watched_entry_unregistered_org() {
    let (env, contract_id, _org, _admin, _webhook) = setup();
    let stranger = Address::generate(&env);
    let client = RegistryContractClient::new(&env, &contract_id);
    let result = client.try_add_watched_entry(
        &stranger,
        &Address::generate(&env),
        &Durability::Persistent,
        &None::<Bytes>,
        &1_000,
        &10_000,
        &true,
    );
    assert_eq!(result, Err(Ok(Error::OrgNotFound)));
}

#[test]
fn test_add_watched_entry_inactive_org() {
    let (env, contract_id, org, _admin, _webhook) = setup();
    let client = RegistryContractClient::new(&env, &contract_id);
    client.deactivate_org(&org);
    let result = client.try_add_watched_entry(
        &org,
        &Address::generate(&env),
        &Durability::Persistent,
        &None::<Bytes>,
        &1_000,
        &10_000,
        &true,
    );
    assert_eq!(result, Err(Ok(Error::OrgInactive)));
}

#[test]
fn test_add_watched_entry_invalid_threshold() {
    let (env, contract_id, org, _admin, _webhook) = setup();
    let client = RegistryContractClient::new(&env, &contract_id);

    // threshold == extend_to is invalid.
    let equal = client.try_add_watched_entry(
        &org,
        &Address::generate(&env),
        &Durability::Persistent,
        &None::<Bytes>,
        &10_000,
        &10_000,
        &true,
    );
    assert_eq!(equal, Err(Ok(Error::InvalidThreshold)));

    // threshold > extend_to is invalid.
    let inverted = client.try_add_watched_entry(
        &org,
        &Address::generate(&env),
        &Durability::Persistent,
        &None::<Bytes>,
        &20_000,
        &10_000,
        &true,
    );
    assert_eq!(inverted, Err(Ok(Error::InvalidThreshold)));
}

#[test]
#[should_panic]
fn test_add_watched_entry_requires_auth() {
    let env = Env::default();
    let contract_id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(&env, &contract_id);
    client.init();
    // No mock_all_auths: require_auth on `org` must fail.
    client.add_watched_entry(
        &Address::generate(&env),
        &Address::generate(&env),
        &Durability::Persistent,
        &None::<Bytes>,
        &1_000,
        &10_000,
        &true,
    );
}
