#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};

//use cw2::set_contract_version;
use cw20;
use cw20_base;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};

//version info for migration info
//const CONTRACT_NAME: &str = "crates.io:xion-minter";
//const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    //set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

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
        admin: None, //TODO :: do I want admin priveliges here
        code_id: msg.cw20_code_id,
        msg: to_json_binary(&cw20_msg)?,
        funds: vec![],
        label: "merger token creation".to_owned(),
    };

    let instantiate_token_submsg =
        SubMsg::reply_on_success(instantiate_msg, INSTANTIATE_TOKEN_REPLY_ID);

    CONFIG.save(
        deps.storage,
        &Config {
            minter: None,
            token_contract: None,
        },
    )?;

    Ok(Response::new()
        .add_submessage(instantiate_token_submsg)
        .add_attribute("action", "instantiate")
        .add_attribute("minter", info.sender))
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
        ExecuteMsg::Mint { amount, recipient } => mint_tokens(deps, env, info, amount, recipient),
    }
}

fn mint_tokens(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    match config.minter {
        None => {
            // First mint attempt - this address becomes the permanent minter
            config.minter = Some(info.sender.clone());
            CONFIG.save(deps.storage, &config.clone())?;
        }
        Some(minter) => {
            // Minter is already set - verify sender has minting rights
            if info.sender != minter {
                return Err(ContractError::Unauthorized {});
            }
        }
    }

    let token_addr = config
        .token_contract
        .ok_or(ContractError::NoContractAddress {})?;

    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    let final_recipient = match recipient {
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
                minter: config.minter.map(|a| a.into_string()),
                token_contract: config.token_contract.map(|a| a.into_string()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Empty;
    use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};

    fn contract_xion_minter() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
        Box::new(contract)
    }

    fn contract_cw20_base() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    fn setup_app() -> (App, Addr, Addr, u64) {
        let mut app = App::default();

        let cw20_code_id = app.store_code(contract_cw20_base());
        let minter_code_id = app.store_code(contract_xion_minter());

        let minter = "the_minter".into_addr();

        let minter_init_msg = InstantiateMsg {
            token_name: "Merger Token".to_string(),
            token_symbol: "MTKN".to_string(),
            token_decimals: 6,
            cw20_code_id: cw20_code_id,
        };

        let minter_addr = app
            .instantiate_contract(
                minter_code_id,
                minter.clone(),
                &minter_init_msg,
                &[],
                "Xion Minter",
                None,
            )
            .unwrap();

        (app, minter, minter_addr, cw20_code_id)
    }

    #[test]
    fn test_minter_instantiates_cw20() {
        let (mut app, minter, minter_addr, _) = setup_app();

        let config_resp: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::GetConfig {})
            .unwrap();

        assert_eq!(config_resp.minter, None);

        let cw20_addr = config_resp.token_contract.expect("No Contract address set");

        let token_info: cw20::TokenInfoResponse = app
            .wrap()
            .query_wasm_smart(&cw20_addr, &cw20::Cw20QueryMsg::TokenInfo {})
            .unwrap();

        assert_eq!(token_info.name, "Merger Token");
        assert_eq!(token_info.symbol, "MTKN");
        assert_eq!(token_info.decimals, 6);
    }

    #[test]
    fn test_mint_tokens() {
        let (mut app, minter, minter_addr, _) = setup_app();

        let recipient = "recipient1".into_addr();

        // Test successful mint by minter
        let mint_msg = ExecuteMsg::Mint {
            amount: Uint128::new(1000),
            recipient: Some(recipient.to_string()),
        };
        app.execute_contract(minter.clone(), minter_addr.clone(), &mint_msg, &[])
            .unwrap();

        let config_resp: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::GetConfig {})
            .unwrap();

        let cw20_addr = config_resp.token_contract.expect("No Contract address set");

        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                &cw20_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: recipient.to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance.balance, Uint128::new(1000));
        assert_eq!(config_resp.minter.unwrap(), minter.into_string());
    }

    #[test]
    fn test_unauthorized_mint() {
        let (mut app, _, minter, _) = setup_app();
        let unauthorized = "unauthorized".into_addr();
        let recipient = "recipient1".into_addr();

        let mint_msg = ExecuteMsg::Mint {
            amount: Uint128::new(1000),
            recipient: Some(recipient.to_string()),
        };

        app.execute_contract(minter.clone(), minter.clone(), &mint_msg, &[])
            .unwrap();

        // Test mint failure from unauthorized address
        let err = app
            .execute_contract(unauthorized, minter.clone(), &mint_msg, &[])
            .unwrap_err();

        match err.downcast::<ContractError>().unwrap() {
            ContractError::Unauthorized {} => {}
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn test_zero_amount_mint() {
        let (mut app, minter, minter_addr, _) = setup_app();
        let recipient = "recipient1".into_addr();

        let mint_msg = ExecuteMsg::Mint {
            amount: Uint128::zero(),
            recipient: Some(recipient.to_string()),
        };

        // Test mint failure with zero amount
        let err = app
            .execute_contract(minter, minter_addr.clone(), &mint_msg, &[])
            .unwrap_err();

        match err.downcast::<ContractError>().unwrap() {
            ContractError::InvalidAmount {} => {}
            e => panic!("unexpected error: {}", e),
        }
    }
}
