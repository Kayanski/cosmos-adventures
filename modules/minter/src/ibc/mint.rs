use crate::{
    contract::{Minter, MinterResult, MINTER_ID},
    msg::MinterIbcMsg,
    state::CONFIG,
};
use abstract_adapter::std::{
    ibc::ModuleIbcMsg,
    objects::{chain_name::ChainName, AccountId},
};
use abstract_ibc_host::endpoints::packet::client_to_host_account_id;
use abstract_sdk::{AccountVerification, ModuleInterface};
use cosmos_adventures_hub::{
    contract::HUB_ID,
    msg::{HubExecuteMsg, NextTokenIdResponse},
};
use cosmwasm_std::{from_json, wasm_execute, DepsMut, Env, Response};

pub fn receive_module_ibc(
    deps: DepsMut,
    env: Env,
    app: Minter,
    msg: ModuleIbcMsg,
) -> MinterResult<Response> {
    // First we verify the calling module has the right namespace
    // We trust all of our modules that are in the same namespace across IBC
    // This is used for extensions that will share a namespace and have a right to execute actions across the protocol

    if msg.source_module.id().ne(MINTER_ID) {
        return Err(crate::error::MinterError::Unauthorized {});
    }

    // Now we can receive the IBC message
    let decoded_message: MinterIbcMsg = from_json(&msg.msg)?;

    match decoded_message {
        MinterIbcMsg::IbcMint {
            local_account_id,
            send_back,
        } => internal_ibc_mint_token(
            deps,
            env,
            app,
            msg.client_chain,
            local_account_id,
            send_back,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn internal_ibc_mint_token(
    deps: DepsMut,
    _env: Env,
    mut adapter: Minter,
    client_chain: ChainName,
    account_id: AccountId,
    send_back: bool,
) -> MinterResult {
    // We get the new owner address
    // This corresponds to an distant account or a local account depending on local_account_id.trace
    // We mint a token on the app's local account

    let target_account = client_to_host_account_id(client_chain.clone(), account_id);
    let resolved_account = adapter
        .account_registry(deps.as_ref())?
        .account_base(&target_account)?;

    // We do as if the calling account was the remote account directly
    adapter.target_account = Some(resolved_account.clone());

    // Then we call the hub to mint a new token
    let config = CONFIG.load(deps.storage)?;
    let module_addr = adapter.modules(deps.as_ref()).module_address(HUB_ID)?;
    let mint_msg = wasm_execute(
        &module_addr,
        &cosmos_adventures_hub::msg::ExecuteMsg::Module(
            abstract_adapter::std::adapter::AdapterRequestMsg {
                proxy_address: Some(resolved_account.proxy.to_string()),
                request: HubExecuteMsg::Mint {
                    module_id: MINTER_ID.to_string(),
                    token_uri: config.token_uri_base,
                    metadata: config.metadata_base,
                },
            },
        ),
        vec![],
    )?;

    let send_back_msg = send_back
        .then(|| {
            let next_token_id: NextTokenIdResponse = deps.querier.query_wasm_smart(
                &module_addr,
                &cosmos_adventures_hub::msg::QueryMsg::Module(
                    cosmos_adventures_hub::msg::HubQueryMsg::NextTokenId {},
                ),
            )?;
            wasm_execute(
                &module_addr,
                &cosmos_adventures_hub::msg::ExecuteMsg::Module(
                    abstract_adapter::std::adapter::AdapterRequestMsg {
                        proxy_address: Some(resolved_account.proxy.to_string()),
                        request: HubExecuteMsg::IbcTransfer {
                            token_id: next_token_id.next_token_id,
                            recipient_chain: client_chain.to_string(),
                        },
                    },
                ),
                vec![],
            )
        })
        .transpose()?;

    Ok(Response::new()
        .add_message(mint_msg)
        .add_messages(send_back_msg))
}
