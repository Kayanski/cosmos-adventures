use crate::contract::{Hub, HubResult};
use crate::error::HubError;
use crate::helpers::next_token_id_mut;
use crate::ibc::TRANSFER_CALLBACK;
use crate::msg::{HubExecuteMsg, HubIbcCallbackMsg, HubIbcMsg};
use crate::state::NFT;
use abstract_core::ibc::CallbackInfo;
use abstract_core::ibc_client::InstalledModuleIdentification;
use abstract_core::objects::module::ModuleInfo;
use abstract_core::{ibc_client, IBC_CLIENT};
use abstract_sdk::features::{AccountIdentification, ModuleIdentification};
use abstract_sdk::{AbstractResponse, AccountAction, Execution, ModuleInterface};
use common::NAMESPACE;
use cosmwasm_std::{ensure_eq, to_json_binary, wasm_execute, DepsMut, Env, MessageInfo};
use cw721::{NftInfoResponse, OwnerOfResponse};
use cw721_metadata_onchain::{ExecuteMsg, Metadata};
use cw721_metadata_onchain::{Extension, QueryMsg};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: Hub,
    msg: HubExecuteMsg,
) -> HubResult {
    match msg {
        HubExecuteMsg::IbcTransfer {
            recipient_chain,
            token_id,
        } => ibc_transfer(deps, info, env, adapter, token_id, recipient_chain),
        HubExecuteMsg::Mint {
            module_id,
            token_uri,
            metadata,
        } => mint(deps, info, env, module_id, token_uri, metadata, adapter),
        HubExecuteMsg::ModifyMetadata {} => todo!(),
    }
}

fn ibc_transfer(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    hub: Hub,
    token_id: String,
    recipient_chain: String,
) -> HubResult {
    let nft = NFT.load(deps.storage)?;

    // We authenticate the account that is calling the contract
    let target_account = hub.account_base(deps.as_ref())?;

    // We verify the NFT is owned by the addr
    let owner: OwnerOfResponse = deps.querier.query_wasm_smart(
        &nft,
        &QueryMsg::OwnerOf {
            token_id: token_id.clone(),
            include_expired: None,
        },
    )?;
    if owner.owner != target_account.proxy_address(deps.as_ref())? {
        return Err(HubError::Unauthorized {});
    }

    // We transfer the NFT from the top level owner to this contract to lock it
    let nft_msg = hub
        .executor(deps.as_ref())
        .execute(vec![AccountAction::from_vec(vec![wasm_execute(
            &nft,
            &ExecuteMsg::TransferNft {
                recipient: env.contract.address.to_string(),
                token_id: token_id.clone(),
            },
            vec![],
        )?])])?;

    // We send an IBC mint message from to the distant chain
    // We query the NFT metadata
    let nft: NftInfoResponse<Extension> = deps.querier.query_wasm_smart(
        NFT.load(deps.storage)?,
        &QueryMsg::NftInfo {
            token_id: token_id.clone(),
        },
    )?;

    let current_module_info = ModuleInfo::from_id(hub.module_id(), hub.version().into())?;
    let ibc_msg = ibc_client::ExecuteMsg::ModuleIbcAction {
        host_chain: recipient_chain,
        source_module: InstalledModuleIdentification {
            module_info: current_module_info.clone(),
            account_id: None,
        },
        target_module: current_module_info,
        msg: to_json_binary(&HubIbcMsg::IbcMint {
            token_id: token_id.clone(),
            token_uri: nft.token_uri,
            extension: nft.extension,
            local_account_id: target_account.account_id(deps.as_ref())?,
        })?,
        callback_info: Some(CallbackInfo {
            id: TRANSFER_CALLBACK.to_string(),
            msg: Some(to_json_binary(&HubIbcCallbackMsg::BurnToken { token_id })?),
        }),
    };

    let ibc_client_addr = hub.modules(deps.as_ref()).module_address(IBC_CLIENT)?;

    // We will burn the token once the transfer has been confirmed and the callback has been received
    let ibc_msg = wasm_execute(ibc_client_addr, &ibc_msg, vec![])?;

    Ok(hub
        .response("ibc-transfer")
        .add_message(nft_msg)
        .add_message(ibc_msg))
}

fn mint(
    mut deps: DepsMut,
    info: MessageInfo,
    env: Env,
    module_id: String,
    token_uri: String,
    metadata: Metadata,
    adapter: Hub,
) -> HubResult {
    // This endpoint is permissionned because we're the hub, only authorized installed modules can call this
    let module_addr = adapter.modules(deps.as_ref()).module_address(&module_id)?;
    ensure_eq!(module_addr, info.sender, HubError::Unauthorized {});
    let namespace = ModuleInfo::from_id_latest(&module_id)?.namespace;
    ensure_eq!(namespace.as_str(), NAMESPACE, HubError::WrongNamespace {});

    let account_base = adapter.account_base(deps.as_ref())?;

    // We mint the token to the recipient
    let token_id = next_token_id_mut(deps.branch(), env)?;
    let mint_msg = wasm_execute(
        NFT.load(deps.storage)?,
        &ExecuteMsg::Mint(cw721_base::MintMsg {
            token_id,
            owner: account_base.proxy.to_string(),
            token_uri: Some(token_uri),
            extension: Some(metadata),
        }),
        vec![],
    )?;

    Ok(adapter.response("mint-lost-nft").add_message(mint_msg))
}
