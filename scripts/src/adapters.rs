use abstract_client::AbstractClient;
use abstract_client::Account;

use abstract_client::Namespace;
use cosmos_adventures_hub::{contract::HUB_ID, msg::HubInstantiateMsg, *};
use cosmwasm_std::coin;
use cw721_metadata_onchain::Metadata;
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, prelude::*};
use minter::contract::interface::CosmosAdventuresMinter;
use minter::msg::MinterInstantiateMsg;

use crate::nft::Cw721;
use crate::MINT_COST;
use crate::MINT_DENOM;

pub fn nft_metadata() -> (String, Metadata) {
    let lost_token_uri = "https://link.org".to_string();
    let lost_metadata = Metadata {
        image: None,
        image_data: None,
        external_url: None,
        description: None,
        name: None,
        attributes: None,
        background_color: None,
        animation_url: None,
        youtube_url: None,
    };

    (lost_token_uri, lost_metadata)
}

/// Set up the test environment with an Account that has the App installed
#[allow(clippy::type_complexity)]
pub fn setup_adapters<Chain: CwEnv>(chain: Chain) -> anyhow::Result<AbstractClient<Chain>> {
    // Deploy an NFT contract
    let nft = Cw721::new("nft_metadata", chain.clone());
    nft.upload()?;

    let namespace = Namespace::from_id(HUB_ID)?;

    // You can set up Abstract with a builder.
    let client = AbstractClient::builder(chain.clone()).build()?;

    // Build a Publisher Account
    let publisher = client
        .publisher_builder(namespace)
        .install_on_sub_account(false)
        .build()?;

    // We publish the HUB
    publisher.publish_adapter::<_, CosmosAdventuresHub<_>>(HubInstantiateMsg {
        nft_code_id: nft.code_id()?,
        admin_account: publisher.account().id()?,
    })?;

    // We publish the Minter
    let (token_uri_base, metadata_base) = nft_metadata();
    publisher.publish_adapter::<_, CosmosAdventuresMinter<_>>(MinterInstantiateMsg {
        admin_account: publisher.account().id()?,
        metadata_base,
        token_uri_base: format!("{token_uri_base}/{}", chain.block_info().unwrap().chain_id),
        mint_limit: 1,
        mint_cost: coin(MINT_COST, MINT_DENOM),
    })?;

    Ok(client)
}

/// Set up the test environment with an Account that has the App installed
#[allow(clippy::type_complexity)]
pub fn setup_account<Chain: CwEnv>(
    client: &AbstractClient<Chain>,
) -> anyhow::Result<Account<Chain>> {
    let account = client
        .account_builder()
        .install_on_sub_account(false)
        .build()?;

    account.set_ibc_status(true)?;

    Ok(account)
}
