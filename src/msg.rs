use abstract_core::objects::AccountId;
use cosmwasm_schema::QueryResponses;

use crate::contract::Hub;
use cw721_metadata_onchain::{Extension, Metadata};

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_adapter::adapter_msg_types!(Hub, HubExecuteMsg, HubQueryMsg);

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct HubInstantiateMsg {
    pub admin_account: AccountId,
    pub nft_code_id: u64,
    pub lost_token_uri: String,
    pub lost_metadata: Metadata,
}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
#[cfg_attr(feature = "interface", impl_into(ExecuteMsg))]
pub enum HubExecuteMsg {
    /// Transfer the NFT cross-chain
    IbcTransfer {
        token_id: String,
        recipient_chain: String,
    },

    /// Mint a new lost token on this contract
    Mint {},
}

#[cosmwasm_schema::cw_serde]
pub enum HubIbcMsg {
    /// Mint a new NFT on the chain from an IBC transfer
    IbcMint {
        local_account_id: AccountId,
        token_id: String,
        token_uri: Option<String>,
        extension: Extension,
    },
}

#[cosmwasm_schema::cw_serde]
pub enum HubIbcCallbackMsg {
    BurnToken { token_id: String },
}

/// App query messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
#[cfg_attr(feature = "interface", impl_into(QueryMsg))]
#[derive(QueryResponses)]
pub enum HubQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
}

#[cosmwasm_schema::cw_serde]
pub enum HubMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {
    pub nft: String,
    pub next_token_id: u64,
}

#[cosmwasm_schema::cw_serde]
pub struct CountResponse {
    pub count: i32,
}
