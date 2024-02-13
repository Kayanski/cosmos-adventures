use abstract_core::{
    ibc_client,
    ibc_host::HostAction,
    manager::{self, ModuleInstallConfig},
    objects::{
        account::AccountTrace,
        chain_name::ChainName,
        module::{ModuleInfo, ModuleVersion},
        AccountId,
    },
    proxy, PROXY,
};
use abstract_sdk::{features::AbstractRegistryAccess, AbstractResponse};
use cosmwasm_std::{to_json_binary, wasm_execute, DepsMut, Env, Response};
use cw721_base::MintMsg;

use crate::{
    contract::{App, AppResult, APP_ID},
    msg::InternalExecuteMsg,
    state::{ACCOUNT, NFT, WHITELISTED_ACCOUNTS},
};
use cw721_metadata_onchain::ExecuteMsg;
use cw721_metadata_onchain::Extension;

pub fn internal_handler(
    deps: DepsMut,
    env: Env,
    adapter: App,
    msg: InternalExecuteMsg,
) -> AppResult {
    match msg {
        InternalExecuteMsg::IbcMint {
            token_id,
            token_uri,
            extension,
            local_chain,
            local_account_id,
        } => internal_ibc_mint_token(
            deps,
            env,
            adapter,
            token_id,
            local_chain,
            local_account_id,
            token_uri,
            extension,
        ),
        InternalExecuteMsg::Whitelist { account } => whitelist(deps, adapter, account),
        InternalExecuteMsg::RemoveWhitelist { account } => remove_whitelist(deps, adapter, account),
        InternalExecuteMsg::Connect { chain } => connect_remote_chain(deps, adapter, chain),
    }
}

#[allow(clippy::too_many_arguments)]
fn internal_ibc_mint_token(
    deps: DepsMut,
    env: Env,
    adapter: App,
    token_id: String,
    local_chain: ChainName,
    local_account_id: AccountId,
    token_uri: Option<String>,
    extension: Extension,
) -> AppResult {
    // We get the new owner address
    // This corresponds to an distant account or a local account depending on local_account_id.trace
    let mut trace = local_account_id.trace().clone();
    let new_trace = match trace.clone() {
        AccountTrace::Local => AccountTrace::Remote(vec![local_chain]),
        AccountTrace::Remote(mut trace_vec) => {
            if trace_vec
                .last()
                .unwrap()
                .eq(&ChainName::from_chain_id(&env.block.chain_id))
            {
                trace_vec.pop();
                AccountTrace::Remote(trace_vec)
            } else {
                trace.push_chain(local_chain);
                trace.clone()
            }
        }
    };

    let target_account_id = AccountId::new(local_account_id.seq(), new_trace)?;

    let recipient = adapter
        .abstract_registry(deps.as_ref())?
        .account_base(&target_account_id, &deps.querier)?
        .proxy;

    // The admin of the NFT is the contract here
    let msg = wasm_execute(
        NFT.load(deps.storage)?,
        &ExecuteMsg::Mint(MintMsg {
            token_id,
            owner: recipient.to_string(),
            token_uri,
            extension,
        }),
        vec![],
    )?;

    Ok(Response::new().add_message(msg))
}

fn whitelist(deps: DepsMut, adapter: App, account: AccountId) -> AppResult {
    WHITELISTED_ACCOUNTS.save(deps.storage, account.clone(), &true)?;

    Ok(adapter
        .response("whitelist-account")
        .add_attribute("account-id", account.to_string()))
}

fn remove_whitelist(deps: DepsMut, adapter: App, account: AccountId) -> AppResult {
    WHITELISTED_ACCOUNTS.save(deps.storage, account.clone(), &false)?;

    Ok(adapter
        .response("remove-whitelist-account")
        .add_attribute("account-id", account.to_string()))
}

fn connect_remote_chain(deps: DepsMut, adapter: App, chain: ChainName) -> AppResult {
    // We install the adapter on the remote contract account, that's all that's needed
    let host_action = HostAction::Dispatch {
        manager_msg: abstract_core::manager::ExecuteMsg::InstallModules {
            modules: vec![ModuleInstallConfig::new(
                ModuleInfo::from_id(APP_ID, ModuleVersion::Latest)?,
                None,
            )],
        },
    };

    let msg = proxy::ExecuteMsg::IbcAction {
        msgs: vec![ibc_client::ExecuteMsg::RemoteAction {
            host_chain: chain.to_string(),
            action: host_action,
            callback_info: None,
        }],
    };

    let manager = adapter
        .abstract_registry(deps.as_ref())?
        .account_base(&ACCOUNT.load(deps.storage)?.account_id, &deps.querier)?
        .manager;

    Ok(adapter
        .response("connect-remote-chain")
        .add_message(wasm_execute(
            manager,
            &manager::ExecuteMsg::ExecOnModule {
                module_id: PROXY.to_string(),
                exec_msg: to_json_binary(&msg)?,
            },
            vec![],
        )?))
}
