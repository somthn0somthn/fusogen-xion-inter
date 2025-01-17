use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid Token: only accepting specified merger tokens")]
    InvalidToken {},

    #[error("Invalid Message")]
    InvalidMessage {},

    #[error("Serialization Error")]
    SerializationError {},
}