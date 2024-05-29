use abstract_adapter::std::objects::AccountId;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use cw721_metadata_onchain::Metadata;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub admin_account: AccountId,
    pub metadata_base: Metadata,
    pub token_uri_base: String,
    pub mint_limit: usize,
    pub mint_cost: Coin,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const CURRENT_MINTED_AMOUNT: Map<&AccountId, usize> = Map::new("minted_amount");
