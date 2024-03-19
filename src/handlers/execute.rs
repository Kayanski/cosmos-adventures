use crate::contract::{Hub, HubResult};
use crate::error::HubError;
use crate::helpers::next_token_id;
use crate::ibc::TRANSFER_CALLBACK;
use crate::msg::{HubExecuteMsg, HubIbcCallbackMsg, HubIbcMsg};
use crate::state::{CONFIG, NFT};
use abstract_core::ibc::CallbackInfo;
use abstract_core::ibc_client::InstalledModuleIdentification;
use abstract_core::objects::module::ModuleInfo;
use abstract_core::objects::nested_admin::query_top_level_owner;
use abstract_core::{ibc_client, IBC_CLIENT};
use abstract_sdk::features::{AccountIdentification, ModuleIdentification};
use abstract_sdk::{AbstractResponse, ModuleInterface};
use cosmwasm_std::{to_json_binary, wasm_execute, DepsMut, Env, MessageInfo};
use cw721::{NftInfoResponse, OwnerOfResponse};
use cw721_metadata_onchain::ExecuteMsg;
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
        HubExecuteMsg::Mint {} => mint(deps, info, env, adapter),
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
    let addr = query_top_level_owner(&deps.querier, target_account.manager.clone())?;

    // We verify the NFT is owned by the addr
    let owner: OwnerOfResponse = deps.querier.query_wasm_smart(
        &nft,
        &QueryMsg::OwnerOf {
            token_id: token_id.clone(),
            include_expired: None,
        },
    )?;
    if owner.owner != addr {
        return Err(HubError::Unauthorized {});
    }

    // We transfer the NFT from the top level owner to this contract to lock it
    let nft_msg = wasm_execute(
        &nft,
        &ExecuteMsg::TransferNft {
            recipient: env.contract.address.to_string(),
            token_id: token_id.clone(),
        },
        vec![],
    )?;

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
            receiver: env.contract.address.to_string(),
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

fn mint(mut deps: DepsMut, _info: MessageInfo, env: Env, adapter: Hub) -> HubResult {
    // We make the abstract account pay a little mint fee (very low, but that allows showcasing token transfers)
    let account_base = adapter.account_base(deps.as_ref())?;

    let top_level_owner = query_top_level_owner(&deps.querier, account_base.manager)?;

    // We mint the token to the recipient

    let config = CONFIG.load(deps.storage)?;
    let token_id = next_token_id(deps.branch(), env)?;
    let mint_msg = wasm_execute(
        NFT.load(deps.storage)?,
        &ExecuteMsg::Mint(cw721_base::MintMsg {
            token_id,
            owner: top_level_owner.to_string(),
            token_uri: Some(config.lost_token_uri),
            extension: Some(config.lost_metadata),
        }),
        vec![],
    )?;

    Ok(adapter.response("mint-lost-nft").add_message(mint_msg))
}
