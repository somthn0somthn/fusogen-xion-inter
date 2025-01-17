#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};

use cw2::set_contract_version;
use cw20; // Add this for MinterResponse
use cw20_base; // Add this for InstantiateMsg

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, WHITELIST}; // TODO: do i need to instantiate this in the instantiated fun

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:whitelist-cw20";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1; //I think this effectively functions as an enum
                                               //pub const CW20_ID: u64 = 42; //TODO move this to a .env file

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)?;

    //this calls a separate contract hence why you have to make
    //a separate InstantiateMsg call
    let cw20_msg = cw20_base::msg::InstantiateMsg {
        //TODO : pull these out into variables
        name: msg.token_name,
        symbol: msg.token_symbol,
        decimals: msg.token_decimals,
        initial_balances: vec![],
        mint: Some(cw20::MinterResponse {
            minter: env.contract.address.to_string(),
            cap: None,
        }),
        marketing: None,
    };

    let instantiate_msg = WasmMsg::Instantiate {
        admin: Some(msg.admin.clone()), //TODO :: do I want admin priveliges here
        code_id: msg.token_code_id,
        msg: to_json_binary(&cw20_msg)?,
        funds: vec![],
        label: "factory token creation".to_owned(),
    };

    let instantiate_token_submsg =
        SubMsg::reply_on_success(instantiate_msg, INSTANTIATE_TOKEN_REPLY_ID);

    CONFIG.save(
        deps.storage,
        &Config {
            admin,
            token_contract: None,
        },
    )?;

    Ok(Response::new()
        .add_submessage(instantiate_token_submsg)
        .add_attribute("action", "instantiate")
        .add_attribute("admin", msg.admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_TOKEN_REPLY_ID => handle_instantiate_token_reply(deps, msg),
        _ => Ok(Response::default()),
    }
}

fn handle_instantiate_token_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    if let Some(res) = msg.result.into_result().ok() {
        let contract_address = res
            .events
            .iter()
            .find(|e| e.ty == "instantiate")
            .and_then(|e| {
                e.attributes
                    .iter()
                    .find(|attr| attr.key == "_contract_address")
            })
            .map(|attr| attr.value.clone())
            .ok_or_else(|| ContractError::NoContractAddress {})?;

        let validated_addr = deps.api.addr_validate(&contract_address)?;
        let mut config = CONFIG.load(deps.storage)?;

        config.token_contract = Some(validated_addr.clone());

        CONFIG.save(deps.storage, &config)?;

        return Ok(Response::new()
            .add_attribute("method", "handle_cw20_instantiate_reply")
            .add_attribute("cw20_contract_addr", contract_address));
    }

    Ok(Response::new().add_attribute("action", "handle_instantiate_token_reply"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddToWhiteList { address } => add_to_whitelist(deps, env, info, address),
        ExecuteMsg::RemoveFromWhiteList { address } => {
            remove_from_whitelist(deps, env, info, address)
        }
        ExecuteMsg::Mint { amount, recipient } => mint_tokens(deps, env, info, amount, recipient),
    }
}

fn add_to_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let addr = deps.api.addr_validate(&address)?;
    WHITELIST.save(deps.storage, &addr, &())?;

    Ok(Response::new()
        .add_attribute("action", "add_to_whitelist")
        .add_attribute("whitelist_addr", addr))
}

fn remove_from_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let addr = deps.api.addr_validate(&address)?;

    WHITELIST.remove(deps.storage, &addr);

    Ok(Response::new()
        .add_attribute("action", "remove_from_whitelist")
        .add_attribute("whitelist_addr", addr))
}

fn mint_tokens(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_addr = config
        .token_contract
        .ok_or(ContractError::NoContractAddress {})?;

    if info.sender != config.admin {
        let is_whitelisted = WHITELIST.may_load(deps.storage, &info.sender)?.is_some();
        if !is_whitelisted {
            return Err(ContractError::Unauthorized {});
        }
    }

    let final_recipient = match recipient {
        //TODO : validate address
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => deps.api.addr_validate(&info.sender.to_string())?,
    };

    let cw20_mint_msg = cw20::Cw20ExecuteMsg::Mint {
        recipient: final_recipient.to_string(),
        amount,
    };

    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: token_addr.to_string(),
        msg: to_json_binary(&cw20_mint_msg)?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(cosmwasm_std::CosmosMsg::Wasm(wasm_msg))
        .add_attribute("action", "mint_tokens")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("final_recipient", final_recipient)
        .add_attribute("amount", amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => {
            let config = CONFIG.load(deps.storage)?;
            to_json_binary(&ConfigResponse {
                admin: config.admin.into_string(),
                token_contract: config.token_contract.map(|a| a.into_string()),
            })
        }
        QueryMsg::IsWhitelisted { address } => {
            let addr = deps.api.addr_validate(&address)?;
            let is_whitelisted = WHITELIST.may_load(deps.storage, &addr)?.is_some();
            to_json_binary(&is_whitelisted)
        }
    }
}