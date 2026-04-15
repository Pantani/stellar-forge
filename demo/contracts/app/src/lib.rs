#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Counter,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn init(env: Env, admin: Address) {
        assert!(
            !env.storage().instance().has(&DataKey::Admin),
            "contract already initialized"
        );
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Counter, &0i128);
    }

    pub fn increment(env: Env, by: i128) -> i128 {
        let admin = Self::admin(env.clone());
        admin.require_auth();
        assert!(by > 0, "increment must be positive");
        let next = Self::count(env.clone()) + by;
        env.storage().instance().set(&DataKey::Counter, &next);
        next
    }

    pub fn set_count(env: Env, value: i128) -> i128 {
        let admin = Self::admin(env.clone());
        admin.require_auth();
        env.storage().instance().set(&DataKey::Counter, &value);
        value
    }

    pub fn count(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::Counter).unwrap_or(0)
    }

    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }
}

mod test;
