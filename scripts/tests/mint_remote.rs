use abstract_client::AbstractClient;
use abstract_client::Application;

use abstract_adapter::std::adapter::AdapterBaseMsg;
use abstract_adapter::std::adapter::AdapterRequestMsg;
use abstract_adapter::std::adapter::BaseExecuteMsg;
use abstract_adapter::std::ibc_client;
use abstract_adapter::std::ibc_host::HostAction;
use abstract_adapter::std::manager;
use abstract_adapter::std::manager::ModuleInstallConfig;
use abstract_adapter::std::objects::chain_name::ChainName;
use abstract_adapter::std::objects::AccountId;
use abstract_adapter::std::proxy;
use abstract_adapter::std::PROXY;
use abstract_client::Namespace;
use abstract_interface::Abstract;
use abstract_interface::AbstractAccount;
use abstract_interface::InstallConfig;
use abstract_interface::ManagerExecFns;
use abstract_interface::ProxyExecFns;
use anyhow::Result as AnyResult;
use ca_scripts::account::get_proxy_address;
use ca_scripts::adapters::setup_account;
use ca_scripts::adapters::setup_adapters;
use ca_scripts::ibc::ibc_abstract_setup;
use ca_scripts::nft::Cw721;
use ca_scripts::nft::ExecuteMsgFns;
use ca_scripts::nft::QueryMsgFns as _;
use ca_scripts::MINT_COST;
use ca_scripts::MINT_DENOM;
use cosmos_adventures_hub::msg::ExecuteMsg;
use cosmos_adventures_hub::msg::HubExecuteMsg;
use cosmos_adventures_hub::{
    contract::HUB_ID,
    msg::{ConfigResponse, HubInstantiateMsg},
    *,
};
use cosmwasm_std::coin;
use cosmwasm_std::coins;
use cosmwasm_std::to_json_binary;
use cw721_metadata_onchain::Metadata;
// Use prelude to get all the necessary imports
use cosmwasm_std::Addr;
use cw_orch::{anyhow, prelude::*};
use cw_orch_interchain::interchain::IbcQueryHandler;
use cw_orch_interchain::interchain::InterchainEnv;
use cw_orch_interchain::interchain::MockBech32InterchainEnv;
use minter::contract::interface::CosmosAdventuresMinter;
use minter::contract::MINTER_ID;
use minter::msg::MinterExecuteMsg;
use minter::msg::MinterExecuteMsgFns;
use minter::msg::MinterInstantiateMsg;

fn get_nft<Chain: CwEnv>(c: &CosmosAdventuresHub<Chain>) -> anyhow::Result<Cw721<Chain>> {
    let ConfigResponse {
        nft: nft_address, ..
    } = c.config()?;

    let nft = Cw721::new("nft", c.get_chain().clone());
    nft.set_address(&Addr::unchecked(nft_address));
    Ok(nft)
}

#[test]
fn successful_install() -> anyhow::Result<()> {
    let chain = MockBech32::new("mock");
    let client = setup_adapters(chain.clone())?;
    let account = setup_account(&client)?;

    let hub = account.install_adapter::<CosmosAdventuresHub<_>>(&[])?;
    let hub: CosmosAdventuresHub<_> = hub.module()?;

    // We assert the nft contract is ok and has the right admin
    let nft = get_nft(&hub)?;
    let nft_minter = nft.minter()?;

    assert_eq!(nft_minter.minter, hub.address()?);

    Ok(())
}

#[test]
fn successful_mint() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let interchain = MockBech32InterchainEnv::new(vec![("juno-1", "juno"), ("phoenix-1", "terra")]);
    let juno = interchain.chain("juno-1")?;
    let terra = interchain.chain("phoenix-1")?;

    let src_client = setup_adapters(juno.clone())?;
    let src_account = setup_account(&src_client)?;

    let dst_client = setup_adapters(terra.clone())?;
    let dst_account = setup_account(&dst_client)?;
    ibc_abstract_setup(&interchain, "juno-1", "phoenix-1")?;

    // The account executes some actions on their remote account :
    // - Install Hub and Minter adapter
    // - Mint a token
    // We verify the nft exists and has been pinted to the right proxy address

    let abstract_account =
        AbstractAccount::new(&Abstract::load_from(juno.clone())?, src_account.id()?);

    let remote_minter_address = CosmosAdventuresMinter::new(MINTER_ID, terra.clone())
        .address()?
        .to_string();

    // We install the necessary modules in the remote account
    let remote_actions_response = abstract_account.manager.execute_on_module(
        PROXY,
        proxy::ExecuteMsg::IbcAction {
            msgs: vec![
                ibc_client::ExecuteMsg::RemoteAction {
                    host_chain: "phoenix".to_string(),
                    action: HostAction::Dispatch {
                        manager_msg: manager::ExecuteMsg::InstallModules {
                            modules: vec![
                                ModuleInstallConfig::new(
                                    CosmosAdventuresMinter::<Mock>::module_info()?,
                                    None,
                                ),
                                ModuleInstallConfig::new(
                                    CosmosAdventuresHub::<Mock>::module_info()?,
                                    None,
                                ),
                            ],
                        },
                    },
                },
                // We authorize the minter module to execute actions on the hub module on behalf of the account
                ibc_client::ExecuteMsg::RemoteAction {
                    host_chain: "phoenix".to_string(),
                    action: HostAction::Dispatch {
                        manager_msg: manager::ExecuteMsg::ExecOnModule {
                            module_id: HUB_ID.to_string(),
                            exec_msg: to_json_binary(&minter::msg::ExecuteMsg::Base(
                                BaseExecuteMsg {
                                    proxy_address: None,
                                    msg: AdapterBaseMsg::UpdateAuthorizedAddresses {
                                        to_add: vec![remote_minter_address],
                                        to_remove: vec![],
                                    },
                                },
                            ))?,
                        },
                    },
                },
            ],
        },
    )?;

    interchain.wait_ibc("juno-1", remote_actions_response)?;

    // We mint the token locally
    let _hub = src_account.install_adapter::<CosmosAdventuresHub<_>>(&[])?;
    let minter = src_account.install_adapter::<CosmosAdventuresMinter<_>>(&[])?;

    juno.add_balance(&src_account.proxy()?, coins(MINT_COST, MINT_DENOM))?;

    let mint_response = minter.execute(
        &minter::msg::ExecuteMsg::Module(AdapterRequestMsg {
            proxy_address: Some(src_account.proxy()?.to_string()),
            request: MinterExecuteMsg::Mint { send_back: false },
        }),
        None,
    )?;

    // And wait for IBC execution
    interchain.wait_ibc("juno-1", mint_response)?;

    // We make sure the distant nft has a minted item that belongs to our proxy
    let distant_hub = dst_account.install_adapter::<CosmosAdventuresHub<_>>(&[])?;

    let nft = get_nft(&distant_hub)?;

    let all_tokens = nft.all_tokens(None, None)?;

    assert_eq!(all_tokens.tokens.len(), 1);
    assert_eq!(all_tokens.tokens[0], "phoenix>0");

    let this_token = nft.owner_of("phoenix>0".to_string(), None)?;
    let proxy_addr = dst_client.account_from(AccountId::remote(
        src_account.id()?.seq(),
        vec![ChainName::from_chain_id("juno-1")],
    )?)?;
    assert_eq!(proxy_addr.proxy()?, this_token.owner);

    Ok(())
}

#[test]
fn successful_mint_send_back() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let interchain = MockBech32InterchainEnv::new(vec![("juno-1", "juno"), ("phoenix-1", "terra")]);
    let juno = interchain.chain("juno-1")?;
    let terra = interchain.chain("phoenix-1")?;

    let src_client = setup_adapters(juno.clone())?;
    let src_account = setup_account(&src_client)?;
    let dst_client = setup_adapters(terra.clone())?;
    let dst_account = setup_account(&dst_client)?;
    ibc_abstract_setup(&interchain, "juno-1", "phoenix-1")?;
    ibc_abstract_setup(&interchain, "phoenix-1", "juno-1")?;

    // The account executes some actions on their remote account :
    // - Install Hub and Minter adapter
    // - Mint a token
    // We verify the nft exists and has been pinted to the right proxy address

    let abstract_account =
        AbstractAccount::new(&Abstract::load_from(juno.clone())?, src_account.id()?);

    let remote_minter_address = CosmosAdventuresMinter::new(MINTER_ID, terra.clone())
        .address()?
        .to_string();

    // We install the necessary modules in the remote account
    let remote_actions_response = abstract_account.manager.execute_on_module(
        PROXY,
        proxy::ExecuteMsg::IbcAction {
            msgs: vec![
                ibc_client::ExecuteMsg::RemoteAction {
                    host_chain: "phoenix".to_string(),
                    action: HostAction::Dispatch {
                        manager_msg: manager::ExecuteMsg::InstallModules {
                            modules: vec![
                                ModuleInstallConfig::new(
                                    CosmosAdventuresMinter::<Mock>::module_info()?,
                                    None,
                                ),
                                ModuleInstallConfig::new(
                                    CosmosAdventuresHub::<Mock>::module_info()?,
                                    None,
                                ),
                            ],
                        },
                    },
                },
                // We authorize the minter module to execute actions on the hub module on behalf of the account
                ibc_client::ExecuteMsg::RemoteAction {
                    host_chain: "phoenix".to_string(),
                    action: HostAction::Dispatch {
                        manager_msg: manager::ExecuteMsg::ExecOnModule {
                            module_id: HUB_ID.to_string(),
                            exec_msg: to_json_binary(
                                &cosmos_adventures_hub::msg::ExecuteMsg::Base(BaseExecuteMsg {
                                    proxy_address: None,
                                    msg: AdapterBaseMsg::UpdateAuthorizedAddresses {
                                        to_add: vec![remote_minter_address],
                                        to_remove: vec![],
                                    },
                                }),
                            )?,
                        },
                    },
                },
                // We authorize IBC operations on the remote account
                ibc_client::ExecuteMsg::RemoteAction {
                    host_chain: "phoenix".to_string(),
                    action: HostAction::Dispatch {
                        manager_msg: manager::ExecuteMsg::UpdateSettings {
                            ibc_enabled: Some(true),
                        },
                    },
                },
            ],
        },
    )?;

    interchain.wait_ibc("juno-1", remote_actions_response)?;

    // We mint the token locally
    let _hub = src_account.install_adapter::<CosmosAdventuresHub<_>>(&[])?;
    let minter = src_account.install_adapter::<CosmosAdventuresMinter<_>>(&[])?;

    juno.add_balance(&src_account.proxy()?, coins(MINT_COST, MINT_DENOM))?;

    let mint_response = minter.execute(
        &minter::msg::ExecuteMsg::Module(AdapterRequestMsg {
            proxy_address: Some(src_account.proxy()?.to_string()),
            request: MinterExecuteMsg::Mint { send_back: true },
        }),
        None,
    )?;

    // And wait for IBC execution
    interchain.wait_ibc("juno-1", mint_response)?;

    // We make sure the distant nft has no item because it was sent back
    let distant_hub = dst_account.install_adapter::<CosmosAdventuresHub<_>>(&[])?;
    let nft = get_nft(&distant_hub)?;
    let all_tokens = nft.all_tokens(None, None)?;
    assert!(all_tokens.tokens.is_empty());

    // We make the local nft has an item that was just transferred

    let src_hub = src_account.application::<CosmosAdventuresHub<_>>()?;
    let nft = get_nft(&src_hub)?;
    let all_tokens = nft.all_tokens(None, None)?;

    assert_eq!(all_tokens.tokens.len(), 1);
    assert_eq!(all_tokens.tokens[0], "phoenix>0");

    let this_token = nft.owner_of("phoenix>0".to_string(), None)?;
    assert_eq!(src_account.proxy()?, this_token.owner);

    Ok(())
}

// let account = AbstractAccount::new(&Abstract::load_from(chain.clone())?, account.id()?);
// let main_account = AbstractAccount::new(
//     &Abstract::load_from(chain.clone())?,
//     adapter.account().id()?,
// );

// let (token_uri, metadata) = nft_metadata();
// // Account can mint
// account.manager.execute_on_module(
//     HUB_ID,
//     ExecuteMsg::Module(AdapterRequestMsg {
//         proxy_address: Some(account.proxy.address()?.to_string()),
//         request: HubExecuteMsg::Mint {
//             module_id: todo!(),
//             token_uri,
//             metadata,
//         },
//     }),
// )?;

// // We verify the token exists
// let nft = get_nft(&adapter.module()?)?;
// let tokens = nft.tokens(chain.sender().to_string(), None, None)?;

// assert_eq!(tokens.tokens.len(), 1);

// let (token_uri, metadata) = nft_metadata();
// // Main Account can mint
// main_account.manager.execute_on_module(
//     HUB_ID,
//     ExecuteMsg::Module(AdapterRequestMsg {
//         proxy_address: Some(main_account.proxy.address()?.to_string()),
//         request: HubExecuteMsg::Mint {
//             token_uri,
//             metadata,
//             module_id: todo!(),
//         },
//     }),
// )?;
// let tokens = nft.tokens(chain.sender().to_string(), None, None)?;

// assert_eq!(tokens.tokens.len(), 2);

//     Ok(())
// }

// #[test]
// fn successful_ibc() -> anyhow::Result<()> {
//     env_logger::init();

//     let interchain = MockBech32InterchainEnv::new(vec![("juno-1", "juno"), ("phoenix-1", "terra")]);
//     let juno = interchain.chain("juno-1")?;
//     let terra = interchain.chain("phoenix-1")?;

//     let client = setup_adapters(juno.clone())?;
//     let (src_adapter,) = setup_account(client)?;
//     let client = setup_adapters(terra.clone())?;
//     let (dst_adapter,) = setup_account(client)?;
//     // We create a new polytone connection between the chains
//     ibc_abstract_setup(&interchain, "juno-1", "phoenix-1")?;

//     let src_account = AbstractAccount::new(
//         &Abstract::load_from(juno.clone())?,
//         src_adapter.account().id()?,
//     );

//     // Create a remote account for the src-account
//     let create_account_response = src_account.manager.execute_on_module(
//         PROXY,
//         proxy::ExecuteMsg::IbcAction {
//             msgs: vec![ibc_client::ExecuteMsg::Register {
//                 host_chain: ChainName::from_chain_id("phoenix-1").to_string(),
//                 base_asset: None,
//                 namespace: None,
//                 install_modules: vec![],
//             }],
//         },
//     )?;

//     interchain.wait_ibc("juno-1", create_account_response)?;

//     let src_nft = get_nft(&src_adapter.module()?)?;
//     let dst_nft = get_nft(&dst_adapter.module()?)?;
//     // We can start working with our app

//     // Src and Dst already have the adapter installed on their account
//     let src_account = AbstractAccount::new(
//         &Abstract::load_from(juno.clone())?,
//         src_adapter.account().id()?,
//     );

//     let (token_uri, metadata) = nft_metadata();
//     src_account.manager.execute_on_module(
//         HUB_ID,
//         ExecuteMsg::Module(AdapterRequestMsg {
//             proxy_address: Some(src_account.proxy.address()?.to_string()),
//             request: HubExecuteMsg::Mint {
//                 token_uri,
//                 metadata,
//                 module_id: todo!(),
//             },
//         }),
//     )?;
//     // We query the token id
//     let token_id = "juno>0".to_string();

//     // We transfer the nft
//     src_nft.approve(src_adapter.address()?.to_string(), token_id.clone(), None)?;

//     let tx_response = src_account.manager.execute_on_module(
//         HUB_ID,
//         ExecuteMsg::Module(AdapterRequestMsg {
//             proxy_address: Some(src_account.proxy.address()?.to_string()),
//             request: HubExecuteMsg::IbcTransfer {
//                 token_id,
//                 recipient_chain: "phoenix".to_string(),
//             },
//         }),
//     )?;

//     interchain.wait_ibc("juno-1", tx_response)?;

//     // We check the token doesn't exist anymore on the contract
//     let tokens = src_nft.all_tokens(None, None)?;
//     assert_eq!(tokens.tokens.len(), 0);

//     // We check the token exists on the remote chain and has the right owner
//     let tokens = dst_nft.all_tokens(None, None)?;
//     assert_eq!(tokens.tokens.len(), 1);

//     Ok(())
// }
