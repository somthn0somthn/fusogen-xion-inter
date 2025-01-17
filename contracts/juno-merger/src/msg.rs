use crate::state::Config;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cw20::Cw20ReceiveMsg;
use cosmwasm_std::{CosmosMsg, Empty, Uint64};
use polytone::{
    ack::Callback,
    callbacks::{CallbackMessage, CallbackRequest},
};

#[cw_serde]
pub struct InstantiateMsg {
    pub note_contract: String,
    pub token_a: String,
    pub token_b: String,
    pub xion_mint_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum ReceiveMsg {
    Lock {
        xion_meta_account: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    GetConfig {},
}

#[cw_serde]
pub enum PolytoneExecuteMsg {
    Execute {
        msgs: Vec<CosmosMsg<Empty>>,
        callback: Option<CallbackRequest>,
        timeout_seconds: Uint64,
    },
}