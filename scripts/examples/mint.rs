use abstract_adapter::std::adapter::AdapterBaseMsg;
use abstract_adapter::std::adapter::AdapterRequestMsg;
use abstract_adapter::std::adapter::BaseExecuteMsg;
use abstract_adapter::std::ibc_client;
use abstract_adapter::std::ibc_host::HostAction;
use abstract_adapter::std::manager;
use abstract_adapter::std::manager::ModuleInstallConfig;
use abstract_adapter::std::proxy;
use abstract_adapter::std::PROXY;
use abstract_interface::Abstract;
use abstract_interface::AbstractAccount;
use abstract_interface::InstallConfig;
use ca_scripts::adapters::setup_account;
use ca_scripts::adapters::setup_adapters;
use ca_scripts::ibc::ibc_abstract_setup;
use ca_scripts::nft::Cw721;
use ca_scripts::nft::QueryMsgFns as _;
use ca_scripts::MINT_COST;
use ca_scripts::MINT_DENOM;
use cosmos_adventures_hub::{contract::HUB_ID, msg::ConfigResponse, *};
use cosmwasm_std::coins;
use cosmwasm_std::to_json_binary;
// Use prelude to get all the necessary imports
use cosmwasm_std::Addr;
use cw_orch::{anyhow, prelude::*};
use cw_orch_interchain::prelude::*;
use minter::contract::interface::CosmosAdventuresMinter;
use minter::contract::MINTER_ID;
use minter::msg::MinterExecuteMsg;

fn get_nft<Chain: CwEnv>(c: &CosmosAdventuresHub<Chain>) -> anyhow::Result<Cw721<Chain>> {
    let ConfigResponse {
        nft: nft_address, ..
    } = c.config()?;

    let nft = Cw721::new("nft", c.get_chain().clone());
    nft.set_address(&Addr::unchecked(nft_address));
    Ok(nft)
}
fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let interchain =
        MockBech32InterchainEnv::new(vec![("juno-1", "juno"), ("stargaze-1", "stargaze")]);
    let juno = interchain.chain("juno-1")?;
    let stargaze = interchain.chain("stargaze-1")?;

    let src_client = setup_adapters(juno.clone())?;
    let src_account = setup_account(&src_client)?;
    let dst_client = setup_adapters(stargaze.clone())?;
    let dst_account = setup_account(&dst_client)?;
    ibc_abstract_setup(&interchain, "juno-1", "stargaze-1")?;
    ibc_abstract_setup(&interchain, "stargaze-1", "juno-1")?;

    // The account executes some actions on their remote account :
    // - Install Hub and Minter adapter
    // - Mint a token
    // We verify the nft exists and has been pinted to the right proxy address

    let abstract_account =
        AbstractAccount::new(&Abstract::load_from(juno.clone())?, src_account.id()?);

    let remote_minter_address = CosmosAdventuresMinter::new(MINTER_ID, stargaze.clone())
        .address()?
        .to_string();

    // We install the necessary modules in the remote account
    let remote_actions_response = abstract_account.manager.execute_on_module(
        PROXY,
        proxy::ExecuteMsg::IbcAction {
            msg: ibc_client::ExecuteMsg::RemoteAction {
                host_chain: "stargaze".to_string(),
                action: HostAction::Dispatch {
                    manager_msgs: vec![
                        manager::ExecuteMsg::InstallModules {
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
                        manager::ExecuteMsg::ExecOnModule {
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
                        manager::ExecuteMsg::UpdateSettings {
                            ibc_enabled: Some(true),
                        },
                    ],
                },
            },
        },
    )?;

    interchain.check_ibc("juno-1", remote_actions_response)?;

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
    interchain.check_ibc("juno-1", mint_response)?;

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
    assert_eq!(all_tokens.tokens[0], "stargaze>0");

    let this_token = nft.owner_of("stargaze>0".to_string(), None)?;
    assert_eq!(src_account.proxy()?, this_token.owner);

    Ok(())
}
