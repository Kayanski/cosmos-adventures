use abstract_adapter::AdapterError;
use abstract_core::{objects::version_control::VersionControlError, AbstractError};
use abstract_sdk::AbstractSdkError;
use cosmwasm_std::{Instantiate2AddressError, StdError};
use cw_asset::AssetError;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum AppError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Abstract(#[from] AbstractError),

    #[error("{0}")]
    AbstractSdk(#[from] AbstractSdkError),

    #[error("{0}")]
    Asset(#[from] AssetError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("{0}")]
    DappError(#[from] AdapterError),

    #[error("{0}")]
    VersionControl(#[from] VersionControlError),

    #[error("{0}")]
    Instantiate2Address(#[from] Instantiate2AddressError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Ibc Transfer failed {0}")]
    Transfer(String),
}
