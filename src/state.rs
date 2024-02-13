use abstract_core::objects::AccountId;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw721_metadata_onchain::Metadata;
use cw_storage_plus::{Item, Map};

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub admin_account: AccountId,
    pub next_token_id: u64,
    pub lost_token_uri: String,
    pub lost_metadata: Metadata,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ACCOUNT: Item<Account> = Item::new("account");
pub const NFT: Item<Addr> = Item::new("nft");
pub const WHITELISTED_ACCOUNTS: Map<AccountId, bool> = Map::new("whitelisted_accounts");

#[cw_serde]
pub struct Account {
    pub account_id: AccountId,
}
