use abstract_core::objects::{chain_name::ChainName, AccountId};
use cosmwasm_schema::QueryResponses;

use crate::contract::App;
use cw721_metadata_onchain::{Extension, Metadata};

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_adapter::adapter_msg_types!(App, AppExecuteMsg, AppQueryMsg);

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct AppInstantiateMsg {
    pub admin_account: AccountId,
    pub nft_code_id: u64,
    pub lost_token_uri: String,
    pub lost_metadata: Metadata,
}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
#[cfg_attr(feature = "interface", impl_into(ExecuteMsg))]
pub enum AppExecuteMsg {
    /// Transfer the NFT cross-chain
    IbcTransfer {
        token_id: String,
        recipient_chain: String,
    },

    /// Mint a new lost token on this contract
    Mint {},

    // Internal actions (mostly for IBC purposes)
    Internal(InternalExecuteMsg),
}

#[cosmwasm_schema::cw_serde]
pub enum InternalExecuteMsg {
    /// Mint a new NFT on the chain from an IBC transfer
    IbcMint {
        token_id: String,
        local_chain: ChainName,
        local_account_id: AccountId,
        token_uri: Option<String>,
        extension: Extension,
    },

    /// Connect with another chain
    Connect { chain: ChainName },

    /// Whitelist an account to execute Internal messages
    Whitelist { account: AccountId },

    /// Remove whitelist for an account to execute Internal messages
    RemoveWhitelist { account: AccountId },
}

#[cosmwasm_schema::cw_serde]
pub enum IbcCallbackMsg {
    BurnToken { token_id: String },
}

/// App query messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
#[cfg_attr(feature = "interface", impl_into(QueryMsg))]
#[derive(QueryResponses)]
pub enum AppQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
}

#[cosmwasm_schema::cw_serde]
pub enum AppMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {
    pub nft: String,
    pub account: AccountId,
    pub next_token_id: u64,
}

#[cosmwasm_schema::cw_serde]
pub struct CountResponse {
    pub count: i32,
}
