use crate::contract::Minter;
use abstract_core::objects::AccountId;
use cosmwasm_schema::QueryResponses;
use cosmwasm_std::Coin;
use cw721_metadata_onchain::Metadata;

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_adapter::adapter_msg_types!(Minter, MinterExecuteMsg, MinterQueryMsg);

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct MinterInstantiateMsg {
    pub admin_account: AccountId,
    pub metadata_base: Metadata,
    pub token_uri_base: String,
    pub mint_limit: usize,
    pub mint_cost: Coin,
}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
#[cfg_attr(feature = "interface", impl_into(ExecuteMsg))]
pub enum MinterExecuteMsg {
    /// Mint a new lost token on this chain.   
    /// This is an endpoint that is callable by any account to mint an NFT.
    Mint {},
}

/// App query messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
#[cfg_attr(feature = "interface", impl_into(QueryMsg))]
#[derive(QueryResponses)]
pub enum MinterQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
}

#[cosmwasm_schema::cw_serde]
pub enum MinterMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {
    pub admin_account: AccountId,
    pub metadata_base: Metadata,
    pub token_uri_base: String,
    pub mint_limit: usize,
    pub mint_cost: Coin,
}

#[cosmwasm_schema::cw_serde]
pub enum MinterIbcMsg {
    IbcMint { local_account_id: AccountId },
}
