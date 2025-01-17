#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, SubMsg, Uint128, Uint64, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use base64;
use serde_json::json;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, PolytoneExecuteMsg, QueryMsg, ReceiveMsg};
use crate::state::{Config, CONFIG};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        note_contract: deps.api.addr_validate(&msg.note_contract)?,
        token_a: deps.api.addr_validate(&msg.token_a)?,
        token_b: deps.api.addr_validate(&msg.token_b)?,
        xion_mint_contract: msg.xion_mint_contract.clone(),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("token_a", msg.token_a)
        .add_attribute("token_b", msg.token_b)
        .add_attribute("note_contract", msg.note_contract)
        .add_attribute("xion_mint_contract", msg.xion_mint_contract))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20_receive) => receive_cw20(deps, env, info, cw20_receive),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.token_a && info.sender != config.token_b {
        return Err(ContractError::InvalidToken {});
    }

    let hook: ReceiveMsg = from_json(&cw20_msg.msg)?;

    match hook {
        ReceiveMsg::Lock { xion_meta_account } => {
            // Format the mint message and base64 encode it
            let msg_str = format!(
                r#"{{"mint":{{"recipient":"{}","amount":"{}"}}}}"#,
                xion_meta_account, cw20_msg.amount
            );
            let base64_msg = base64::encode(msg_str);

            // Create the polytone message exactly like the workshop
            let execute_msg = json!({
                "execute": {
                    "msgs": [{
                        "wasm": {
                            "execute": {
                                "contract_addr": config.xion_mint_contract,
                                "msg": base64_msg,
                                "funds": []
                            }
                        }
                    }],
                    "callback": {
                        "receiver": env.contract.address.to_string(),
                        "msg": base64::encode("mint_complete")
                    },
                    "timeout_seconds": "300"
                }
            });

            // Send to note contract
            let note_msg = WasmMsg::Execute {
                contract_addr: config.note_contract.to_string(),
                msg: to_json_binary(&execute_msg)?,
                funds: vec![],
            };

            Ok(Response::new()
                .add_message(note_msg)
                .add_attribute("action", "lock_and_mint")
                .add_attribute("locked_token", info.sender)
                .add_attribute("from_user", cw20_msg.sender)
                .add_attribute("amount_locked", cw20_msg.amount)
                .add_attribute("xion_recipient", xion_meta_account))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&CONFIG.load(deps.storage)?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{to_binary, Empty, Uint128};
    use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor, IntoAddr};

    use crate::ContractError;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;

    use crate::msg::PolytoneExecuteMsg as MockNoteMsg;

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn mock_note_instantiate(
        _deps: cosmwasm_std::DepsMut,
        _env: cosmwasm_std::Env,
        _info: cosmwasm_std::MessageInfo,
        _msg: Empty,
    ) -> Result<cosmwasm_std::Response, cosmwasm_std::StdError> {
        Ok(cosmwasm_std::Response::new().add_attribute("mock_note", "init"))
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn mock_note_execute(
        _deps: cosmwasm_std::DepsMut,
        _env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        msg: MockNoteMsg,
    ) -> Result<cosmwasm_std::Response, cosmwasm_std::StdError> {
        match msg {
            MockNoteMsg::Execute {
                msgs,
                callback,
                timeout_seconds,
            } => {
                Ok(cosmwasm_std::Response::new()
                    .add_attribute("mock_note", "received_execute")
                    .add_attribute("caller", info.sender.to_string())
                    .add_attribute("msgs_len", msgs.len().to_string())
                    .add_attribute("timeout_seconds", timeout_seconds.to_string())
                    .add_attribute("callback", format!("{:?}", callback)))
            }
        }
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn mock_note_query(
        _deps: cosmwasm_std::Deps<cosmwasm_std::Empty>,
        _env: cosmwasm_std::Env,
        _msg: cosmwasm_std::Binary,
    ) -> Result<cosmwasm_std::Binary, cosmwasm_std::StdError> {
        Ok(to_binary("no queries")?)
    }

    fn mock_note_contract() -> Box<dyn Contract<Empty>> {
        let contract =
            ContractWrapper::new(mock_note_execute, mock_note_instantiate, mock_note_query);
        Box::new(contract)
    }

    fn merger_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query);
        Box::new(contract)
    }

    fn cw20_base_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    fn setup() -> (
        App,
        Addr, 
        Addr, 
        Addr, 
        Addr, 
    ) {
        let mut app = App::default();

        let cw20_code_id = app.store_code(cw20_base_contract());
        let merger_code_id = app.store_code(merger_contract());
        let note_code_id = app.store_code(mock_note_contract());

        // instantiate the mock note
        let note_addr = app
            .instantiate_contract(
                note_code_id,
                "note_deployer".into_addr(),
                &Empty {}, 
                &[],
                "Mock Note",
                None,
            )
            .unwrap();

        // instantiate Token A
        let token_a_admin = "token_a_admin".into_addr();
        let token_a_init = cw20_base::msg::InstantiateMsg {
            name: "Token A".into(),
            symbol: "TKNA".into(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(cw20::MinterResponse {
                minter: token_a_admin.to_string(),
                cap: None,
            }),
            marketing: None,
        };
        let token_a_addr = app
            .instantiate_contract(
                cw20_code_id,
                token_a_admin.clone(),
                &token_a_init,
                &[],
                "Token A",
                None,
            )
            .unwrap();

        let xion_mint_addr = token_a_addr.clone();

        // instantiate the Merger
        let placeholder = "placeholder".into_addr();
        let init_msg = InstantiateMsg {
            note_contract: note_addr.to_string(),
            token_a: token_a_addr.to_string(),
            token_b: placeholder.to_string(),
            xion_mint_contract: xion_mint_addr.to_string(),
        };
        let merger_addr = app
            .instantiate_contract(
                merger_code_id,
                "merger_deployer".into_addr(),
                &init_msg,
                &[],
                "Merger Contract",
                None,
            )
            .unwrap();

        (app, merger_addr, token_a_addr, note_addr, token_a_admin)
    }

    #[test]
    fn test_init() {
        let (app, merger_addr, token_a_addr, note_addr, _) = setup();

        let cfg: super::Config = app
            .wrap()
            .query_wasm_smart(merger_addr, &QueryMsg::GetConfig {})
            .unwrap();

        assert_eq!(cfg.token_a, token_a_addr);
        assert_eq!(cfg.note_contract, note_addr);
    }

    #[test]
    fn test_receive_lock() {
        let (mut app, merger_addr, token_a_addr, _note_addr, token_a_admin) = setup();

        // 1) Mint some "TokenA" for a user
        let user = "user1".into_addr();
        let amount = Uint128::new(500);
        app.execute_contract(
            token_a_admin,
            token_a_addr.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: user.to_string(),
                amount,
            },
            &[],
        )
        .unwrap();

        // 2) user sends token_a to the merger
        let lock_msg = ReceiveMsg::Lock {
            xion_meta_account: "xion1xyz".to_string(),
        };
        let send_msg = Cw20ExecuteMsg::Send {
            contract: merger_addr.to_string(),
            amount,
            msg: to_binary(&lock_msg).unwrap(),
        };

        let res = app
            .execute_contract(user.clone(), token_a_addr.clone(), &send_msg, &[])
            .unwrap();

        // 3) Check for expected events
        let wasm_events: Vec<_> = res.events.iter().filter(|e| e.ty == "wasm").collect();
        assert!(wasm_events.len() >= 2, "Expected at least 2 wasm events");

        let found_lock_and_mint = wasm_events.iter().any(|ev| {
            ev.attributes
                .iter()
                .any(|attr| attr.key == "action" && attr.value == "lock_and_mint")
        });
        assert!(
            found_lock_and_mint,
            "No lock_and_mint action found in events"
        );

        let mock_note_evt = res
            .events
            .iter()
            .find(|ev| ev.attributes.iter().any(|at| at.key == "mock_note"))
            .expect("mock note should have been called");

        let mock_note_attr = mock_note_evt
            .attributes
            .iter()
            .find(|at| at.key == "mock_note")
            .unwrap();
        assert_eq!(mock_note_attr.value, "received_execute");
    }
}