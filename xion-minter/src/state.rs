use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub token_contract: Option<Addr>,
    pub minter: Option<Addr>  //this works as a first-come-first-served b/c I dont see how 
                              //polytone's proxy can instantiate a contract, however the first mint execution call
                              //irrevocably sets to the minter to the caller
}

pub const CONFIG: Item<Config> = Item::new("config");
