use soroban_sdk::{Address, Env, Symbol, contracttype, contractclient};

#[contracttype]
#[derive(Clone)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

#[contracttype]
#[derive(Clone)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contractclient(name = "ReflectorClient")]
pub trait ReflectorOracle {
    fn base(env: &Env) -> Asset;
    fn decimals(env: &Env) -> u32;
    fn lastprice(env: &Env, asset: Asset) -> Option<PriceData>;
}