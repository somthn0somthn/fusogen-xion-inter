use crate::state::Config;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub token_name: String,
    pub token_symbol: String,
    pub token_decimals: u8,
    pub cw20_code_id: u64, //I'm not sure exactly how this works and how best to query this
                            //because it is the code Id of the deployed cw20 smart contract, I believe
}

#[cw_serde]
pub enum ExecuteMsg {
    Mint {
        amount: Uint128,
        recipient: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    GetConfig {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub minter: Option<String>,
    pub token_contract: Option<String>,
}