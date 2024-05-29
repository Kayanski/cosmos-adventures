use abstract_adapter::std::objects::AccountId;
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
    /// This is an authorized endpoint that is only callable by another app in the same namespace
    Mint {
        module_id: String,
        token_uri: String,
        metadata: Metadata,
    },

    /// Change the metadata of an NFT
    /// This is an authorized endpoint that is only callable by another app in the same namespace
    ModifyMetadata {},
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
    #[returns(NextTokenIdResponse)]
    NextTokenId {},
}

#[cosmwasm_schema::cw_serde]
pub enum HubMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {
    pub nft: String,
    pub next_token_id: u64,
}

#[cosmwasm_schema::cw_serde]
pub struct NextTokenIdResponse {
    pub next_token_id: String,
}
