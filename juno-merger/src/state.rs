use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

//TODO - store as Addr or String?
#[cw_serde]
pub struct Config {
    pub note_contract: Addr,      
    pub token_a: Addr,           
    pub token_b: Addr,           
    pub xion_mint_contract: String, 
}

pub const CONFIG: Item<Config> = Item::new("config");