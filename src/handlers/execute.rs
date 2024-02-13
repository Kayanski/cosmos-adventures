use super::internal::internal_handler;
use crate::contract::{App, AppResult, APP_ID};
use crate::error::AppError;
use crate::helpers::next_token_id;
use crate::ibc::TRANSFER_CALLBACK;
use crate::msg::{AppExecuteMsg, IbcCallbackMsg, InternalExecuteMsg};
use crate::state::{ACCOUNT, CONFIG, NFT, WHITELISTED_ACCOUNTS};
use abstract_core::ibc::CallbackInfo;
use abstract_core::ibc_host::HostAction;
use abstract_core::objects::chain_name::ChainName;
use abstract_core::objects::nested_admin::query_top_level_owner;
use abstract_core::{ibc_client, manager, proxy, PROXY};
use abstract_sdk::features::{AbstractRegistryAccess, AccountIdentification};
use abstract_sdk::AbstractResponse;
use cosmwasm_std::{to_json_binary, wasm_execute, DepsMut, Env, MessageInfo};
use cw721::{NftInfoResponse, OwnerOfResponse};
use cw721_metadata_onchain::ExecuteMsg;
use cw721_metadata_onchain::{Extension, QueryMsg};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: App,
    msg: AppExecuteMsg,
) -> AppResult {
    match msg {
        AppExecuteMsg::IbcTransfer {
            recipient_chain,
            token_id,
        } => ibc_transfer(deps, info, env, adapter, token_id, recipient_chain),
        AppExecuteMsg::Mint {} => mint(deps, info, env, adapter),
        AppExecuteMsg::Internal(internal_msg) => {
            // We check the sender is authorized to call internal messages
            let target_account = adapter.account_id(deps.as_ref())?;
            let config = CONFIG.load(deps.storage)?;

            if !WHITELISTED_ACCOUNTS
                .load(deps.storage, target_account)
                .unwrap_or(false)
                && adapter.account_id(deps.as_ref())? != config.admin_account
            {
                return Err(AppError::Unauthorized {});
            }

            internal_handler(deps, env, adapter, internal_msg)
        }
    }
}

fn ibc_transfer(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    adapter: App,
    token_id: String,
    recipient_chain: String,
) -> AppResult {
    let nft = NFT.load(deps.storage)?;

    // We authenticate the account that is calling the contract
    let target_account = adapter.account_base(deps.as_ref())?;
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
        return Err(AppError::Unauthorized {});
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

    let app_msg: crate::msg::ExecuteMsg = AppExecuteMsg::Internal(InternalExecuteMsg::IbcMint {
        token_id: token_id.clone(),
        local_account_id: target_account.account_id(deps.as_ref())?,
        token_uri: nft.token_uri,
        extension: nft.extension,
        local_chain: ChainName::from_chain_id(&env.block.chain_id),
    })
    .into();

    let proxy_msg = proxy::ExecuteMsg::IbcAction {
        msgs: vec![ibc_client::ExecuteMsg::RemoteAction {
            host_chain: recipient_chain,
            action: HostAction::Dispatch {
                manager_msg: manager::ExecuteMsg::ExecOnModule {
                    module_id: APP_ID.to_string(),
                    exec_msg: to_json_binary(&app_msg)?,
                },
            },
            callback_info: Some(CallbackInfo {
                id: TRANSFER_CALLBACK.to_string(),
                msg: Some(to_json_binary(&IbcCallbackMsg::BurnToken { token_id })?),
                receiver: env.contract.address.to_string(),
            }),
        }],
    };

    let ibc_msg = manager::ExecuteMsg::ExecOnModule {
        module_id: PROXY.to_string(),
        exec_msg: to_json_binary(&proxy_msg)?,
    };
    let manager_addr = adapter
        .abstract_registry(deps.as_ref())?
        .account_base(&ACCOUNT.load(deps.storage)?.account_id, &deps.querier)?
        .manager;

    // We will burn the token once the transfer has been confirmed and the callback has been received
    let ibc_msg = wasm_execute(manager_addr, &ibc_msg, vec![])?;

    Ok(adapter
        .response("ibc-transfer")
        .add_message(nft_msg)
        .add_message(ibc_msg))
}

fn mint(mut deps: DepsMut, _info: MessageInfo, env: Env, adapter: App) -> AppResult {
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
