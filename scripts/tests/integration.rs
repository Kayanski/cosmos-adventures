use abstract_client::AbstractClient;
use abstract_client::Application;

use abstract_client::GovernanceDetails;
use abstract_client::Namespace;
use abstract_core::account_factory;
use abstract_core::adapter::AdapterRequestMsg;
use abstract_core::ibc_client;
use abstract_core::objects::chain_name::ChainName;
use abstract_core::objects::AccountId;
use abstract_core::proxy;
use abstract_core::IBC_CLIENT;
use abstract_core::PROXY;
use abstract_interface::Abstract;
use abstract_interface::AbstractAccount;
use abstract_interface::ManagerExecFns;
use abstract_interface::ManagerQueryFns;
use abstract_interface::VCQueryFns;
use ca_scripts::nft::Cw721;
use ca_scripts::nft::ExecuteMsgFns;
use ca_scripts::nft::QueryMsgFns as _;
use cosmos_adventures_hub::msg::AppExecuteMsg;
use cosmos_adventures_hub::msg::ExecuteMsg;
use cosmos_adventures_hub::msg::InternalExecuteMsg;
use cosmos_adventures_hub::{
    contract::APP_ID,
    msg::{AppInstantiateMsg, ConfigResponse},
    *,
};
use cw721_metadata_onchain::Metadata;
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, prelude::*};

use ca_scripts::abstract_ibc::ibc_abstract_setup;
use cosmwasm_std::Addr;
use cw_orch_interchain::interchain::InterchainEnv;
use cw_orch_interchain::interchain::MockInterchainEnv;

fn get_nft<Chain: CwEnv>(c: &CosmosAdventuresHub<Chain>) -> anyhow::Result<Cw721<Chain>> {
    let ConfigResponse {
        nft: nft_address, ..
    } = c.config()?;

    let nft = Cw721::new("nft", c.get_chain().clone());
    nft.set_address(&Addr::unchecked(nft_address));
    Ok(nft)
}

/// Set up the test environment with an Account that has the App installed
#[allow(clippy::type_complexity)]
fn setup<Chain: CwEnv>(
    chain: Chain,
) -> anyhow::Result<(
    AbstractClient<Chain>,
    Application<Chain, CosmosAdventuresHub<Chain>>,
)> {
    // Deploy an NFT contract
    let nft = Cw721::new("nft_metadata", chain.clone());
    nft.upload()?;

    let namespace = Namespace::from_id(APP_ID)?;

    // You can set up Abstract with a builder.
    let client = AbstractClient::builder(chain.clone()).build()?;

    // Build a Publisher Account
    let publisher = client
        .publisher_builder(namespace)
        .install_on_sub_account(false)
        .build()?;

    publisher.publish_adapter::<_, CosmosAdventuresHub<_>>(AppInstantiateMsg {
        nft_code_id: nft.code_id()?,
        lost_token_uri: "https://link.org".to_string(),
        lost_metadata: Metadata {
            image: None,
            image_data: None,
            external_url: None,
            description: None,
            name: None,
            attributes: None,
            background_color: None,
            animation_url: None,
            youtube_url: None,
        },
        admin_account: publisher.account().id()?,
    })?;

    let app = publisher
        .account()
        .install_adapter::<CosmosAdventuresHub<_>>(&[])?;

    let account = AbstractAccount::new(
        &Abstract::load_from(chain.clone())?,
        publisher.account().id()?,
    );
    account.manager.update_settings(Some(true))?;

    Ok((client, app))
}

#[test]
fn successful_install() -> anyhow::Result<()> {
    let chain = Mock::new(&Addr::unchecked("sender"));
    let (_, adapter) = setup(chain.clone())?;
    let adapter: CosmosAdventuresHub<_> = adapter.module()?;

    let ConfigResponse { account, .. } = adapter.config()?;

    // We assert the create account is the right one, have has the right owner
    let account_base = AbstractAccount::new(&Abstract::load_from(chain.clone())?, account);
    let account_info = account_base.manager.info()?.info;
    if let GovernanceDetails::Monarchy { monarch } = &account_info.governance_details {
        if adapter.address()? != monarch {
            panic!(
                "Expected contract controlled governance : {:?}",
                account_info
            )
        }
    } else {
        panic!(
            "Expected contract controlled governance : {:?}",
            account_info
        )
    }

    // We assert the nft contract is ok and has the right admin
    let nft = get_nft(&adapter)?;
    let nft_minter = nft.minter()?;

    assert_eq!(nft_minter.minter, adapter.address()?);

    Ok(())
}

#[test]
fn successful_mint() -> anyhow::Result<()> {
    let chain = Mock::new(&Addr::unchecked("sender"));
    let (client, adapter) = setup(chain.clone())?;

    // We create an account which will mint a token
    let account = client
        .account_builder()
        .install_on_sub_account(false)
        .build()?;
    account.install_adapter::<CosmosAdventuresHub<_>>(&[])?;
    let account = AbstractAccount::new(&Abstract::load_from(chain.clone())?, account.id()?);
    let main_account = AbstractAccount::new(
        &Abstract::load_from(chain.clone())?,
        adapter.account().id()?,
    );

    // Account can mint
    account.manager.execute_on_module(
        APP_ID,
        ExecuteMsg::Module(AdapterRequestMsg {
            proxy_address: Some(account.proxy.address()?.to_string()),
            request: AppExecuteMsg::Mint {},
        }),
    )?;

    // We verify the token exists
    let nft = get_nft(&adapter.module()?)?;
    let tokens = nft.tokens(chain.sender().to_string(), None, None)?;

    assert_eq!(tokens.tokens.len(), 1);

    // Main Account can mint
    main_account.manager.execute_on_module(
        APP_ID,
        ExecuteMsg::Module(AdapterRequestMsg {
            proxy_address: Some(main_account.proxy.address()?.to_string()),
            request: AppExecuteMsg::Mint {},
        }),
    )?;
    let tokens = nft.tokens(chain.sender().to_string(), None, None)?;

    assert_eq!(tokens.tokens.len(), 2);

    Ok(())
}

#[test]
fn successful_ibc() -> anyhow::Result<()> {
    env_logger::init();

    let sender = &Addr::unchecked("sender");
    let interchain = MockInterchainEnv::new(vec![("juno-1", sender), ("phoenix-1", sender)]);
    let juno = interchain.chain("juno-1")?;
    let terra = interchain.chain("phoenix-1")?;

    let (_, src_adapter) = setup(juno.clone())?;
    let (_, dst_adapter) = setup(terra.clone())?;
    // We create a new polytone connection between the chains
    ibc_abstract_setup(&interchain, "juno-1", "phoenix-1")?;

    // We register the src account as a whitelist address on the dst chain
    let contract_account_id = src_adapter.config()?.account;
    let src_account = AbstractAccount::new(
        &Abstract::load_from(juno.clone())?,
        src_adapter.account().id()?,
    );
    let dst_account = AbstractAccount::new(
        &Abstract::load_from(terra.clone())?,
        dst_adapter.account().id()?,
    );

    // Create a remote account for the src-account
    let create_account_response = src_account.manager.execute_on_module(
        PROXY,
        proxy::ExecuteMsg::IbcAction {
            msgs: vec![ibc_client::ExecuteMsg::Register {
                host_chain: ChainName::from_chain_id("phoenix-1").to_string(),
                base_asset: None,
                namespace: None,
                install_modules: vec![],
            }],
        },
    )?;

    interchain.wait_ibc(&"juno-1".to_string(), create_account_response)?;

    // Install the adapter on the src adapter account
    let register_response = src_account.manager.execute_on_module(
        APP_ID,
        ExecuteMsg::Module(AdapterRequestMsg {
            proxy_address: Some(dst_account.proxy.address()?.to_string()),
            request: AppExecuteMsg::Internal(InternalExecuteMsg::Connect {
                chain: ChainName::from_chain_id("phoenix-1"),
            }),
        }),
    )?;

    interchain.wait_ibc(&"juno-1".to_string(), register_response)?;

    // Whitelist the src adapter account on the dst_adapter account
    dst_account.manager.execute_on_module(
        APP_ID,
        ExecuteMsg::Module(AdapterRequestMsg {
            proxy_address: Some(dst_account.proxy.address()?.to_string()),
            request: AppExecuteMsg::Internal(InternalExecuteMsg::Whitelist {
                account: AccountId::remote(
                    contract_account_id.seq(),
                    vec![ChainName::from_chain_id("juno-1")],
                )?,
            }),
        }),
    )?;

    let src_nft = get_nft(&src_adapter.module()?)?;
    let dst_nft = get_nft(&dst_adapter.module()?)?;
    // We can start working with our app

    // Src and Dst already have the adapter installed on their account
    let src_account = AbstractAccount::new(
        &Abstract::load_from(juno.clone())?,
        src_adapter.account().id()?,
    );

    src_account.manager.execute_on_module(
        APP_ID,
        ExecuteMsg::Module(AdapterRequestMsg {
            proxy_address: Some(src_account.proxy.address()?.to_string()),
            request: AppExecuteMsg::Mint {},
        }),
    )?;
    // We query the token id
    let token_id = "juno>0".to_string();

    // We transfer the nft
    src_nft.approve(src_adapter.address()?.to_string(), token_id.clone(), None)?;

    let tx_response = src_account.manager.execute_on_module(
        APP_ID,
        ExecuteMsg::Module(AdapterRequestMsg {
            proxy_address: Some(src_account.proxy.address()?.to_string()),
            request: AppExecuteMsg::IbcTransfer {
                token_id,
                recipient_chain: "phoenix".to_string(),
            },
        }),
    )?;

    interchain.wait_ibc(&"juno-1".to_string(), tx_response)?;

    // We check the token doesn't exist anymore on the contract
    let tokens = src_nft.all_tokens(None, None)?;
    assert_eq!(tokens.tokens.len(), 0);

    // We check the token exists on the remote chain and has the right owner
    let tokens = dst_nft.all_tokens(None, None)?;
    assert_eq!(tokens.tokens.len(), 1);

    Ok(())
}

// #[test]
// fn successful_reset() -> anyhow::Result<()> {
//     let (_, app) = setup(0)?;

//     app.reset(42)?;
//     let count: CountResponse = app.count()?;
//     assert_eq!(count.count, 42);
//     Ok(())
// }

// #[test]
// fn failed_reset() -> anyhow::Result<()> {
//     let (_, app) = setup(0)?;

//     let err: AppError = app
//         .call_as(&Addr::unchecked("NotAdmin"))
//         .reset(9)
//         .unwrap_err()
//         .downcast()
//         .unwrap();
//     assert_eq!(err, AppError::Admin(AdminError::NotAdmin {}));
//     Ok(())
// }
