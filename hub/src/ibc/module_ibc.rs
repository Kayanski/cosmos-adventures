use crate::{
    contract::{Hub, HubResult},
    msg::HubIbcMsg,
    state::NFT,
};
use abstract_adapter::std::{
    ibc::ModuleIbcMsg,
    objects::{chain_name::ChainName, AccountId},
};
use abstract_ibc_host::endpoints::packet::client_to_host_account_id;
use abstract_sdk::AccountVerification;
use common::NAMESPACE;
use cosmwasm_std::{from_json, wasm_execute, DepsMut, Env, Response};
use cw721_base::MintMsg;
use cw721_metadata_onchain::{ExecuteMsg, Extension};

pub fn receive_module_ibc(
    deps: DepsMut,
    env: Env,
    app: Hub,
    msg: ModuleIbcMsg,
) -> HubResult<Response> {
    // First we verify the calling module has the right namespace
    // We trust all of our modules that are in the same namespace across IBC
    // This is used for extensions that will share a namespace and have a right to execute actions across the protocol

    if msg.source_module.namespace.as_str().ne(NAMESPACE) {
        return Err(crate::error::HubError::Unauthorized {});
    }

    // Now we can receive the IBC message
    let decoded_message: HubIbcMsg = from_json(&msg.msg)?;

    match decoded_message {
        HubIbcMsg::IbcMint {
            token_id,
            token_uri,
            extension,
            local_account_id,
        } => internal_ibc_mint_token(
            deps,
            env,
            app,
            msg.client_chain,
            local_account_id,
            token_id,
            token_uri,
            extension,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn internal_ibc_mint_token(
    deps: DepsMut,
    env: Env,
    hub: Hub,
    client_chain: ChainName,
    account_id: AccountId,
    token_id: String,
    token_uri: Option<String>,
    extension: Extension,
) -> HubResult {
    // We get the new owner address
    // This corresponds to an distant account or a local account depending on local_account_id.trace
    // We mint a token on the app's local account
    let target_account = if account_id.is_remote() {
        match account_id.trace() {
            abstract_adapter::std::objects::account::AccountTrace::Local => unreachable!(),
            abstract_adapter::std::objects::account::AccountTrace::Remote(trace) => {
                if trace.last() == Some(&ChainName::from_chain_id(&env.block.chain_id)) {
                    let mut new_trace = trace.clone();
                    new_trace.pop();
                    if new_trace.is_empty() {
                        AccountId::local(account_id.seq())
                    } else {
                        AccountId::remote(account_id.seq(), new_trace)?
                    }
                } else {
                    client_to_host_account_id(client_chain.clone(), account_id.clone())
                }
            }
        }
    } else {
        client_to_host_account_id(client_chain.clone(), account_id.clone())
    };

    let resolved_account = hub
        .account_registry(deps.as_ref())?
        .account_base(&target_account)?;

    // The admin of the NFT is the contract here
    let msg = wasm_execute(
        NFT.load(deps.storage)?,
        &ExecuteMsg::Mint(MintMsg {
            token_id,
            owner: resolved_account.proxy.to_string(),
            token_uri,
            extension,
        }),
        vec![],
    )?;

    Ok(Response::new().add_message(msg))
}
