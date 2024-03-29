use std::any::type_name;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use secret_toolkit::{
    serialization::{Bincode2, Serde},
};


use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage};
//use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};



pub static CONFIG_KEY: &[u8] = b"config";
pub static PRNG_SEED_KEY: &[u8] = b"prng";

pub const PREFIX_TOKEN_CONTRACT_INFO: &[u8] = b"tokeninfo";
pub const PREFIX_ALIAS_TO_ADDR: &[u8] = b"aliasaddr";
pub const PREFIX_CUSTOM_ALIAS: &[u8] = b"custom";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    // Permissions to edit rates
    pub admin: CanonicalAddr,
    // Marks whether txs are allowed to be sent
    pub active: bool,


    // Percentage of tx taken as fee
    pub fee: u128,

    // Decimals of percentage
    pub fee_decimals: u8,



}



/// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
/// StdError::NotFound if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Bincode2::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}


/// Returns StdResult<Option<T>> from retrieving the item with the specified key.
/// Returns Ok(None) if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn may_load<T: DeserializeOwned, S: ReadonlyStorage>(
    storage: &S,
    key: &[u8],
) -> StdResult<Option<T>> {
    match storage.get(key) {
        Some(value) => Bincode2::deserialize(&value).map(Some),
        None => Ok(None),
    }
}



/// Returns StdResult<()> resulting from saving an item to storage
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item should go to
/// * `key` - a byte slice representing the key to access the stored item
/// * `value` - a reference to the item to store
pub fn save<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], value: &T) -> StdResult<()> {
    storage.set(key, &Bincode2::serialize(value)?);
    Ok(())
}



/// Removes an item from storage
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn remove<S: Storage>(storage: &mut S, key: &[u8]) {
    storage.remove(key);
}
