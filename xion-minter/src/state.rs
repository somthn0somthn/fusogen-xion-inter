use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub token_contract: Option<Addr>,
    pub minter: Addr, //this will be the initiator that has the permissions to mint via the associated cw20
}

pub const CONFIG: Item<Config> = Item::new("config");
