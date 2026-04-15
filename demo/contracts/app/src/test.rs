#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn starter_contract_tracks_admin_and_counter() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    client.init(&admin);
    assert_eq!(client.admin(), admin);
    assert_eq!(client.count(), 0);
    assert_eq!(client.increment(&4), 4);
    assert_eq!(client.set_count(&11), 11);
    assert_eq!(client.count(), 11);
}
