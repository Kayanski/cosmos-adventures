use abstract_core::ibc::{CallbackResult, IbcCallbackMsg};
use abstract_sdk::AbstractResponse;
use cosmwasm_std::{from_json, wasm_execute, DepsMut, Env, MessageInfo};

use crate::{
    contract::{Hub, HubResult},
    error::HubError,
    msg::HubIbcCallbackMsg,
    state::NFT,
};
use cw721_metadata_onchain::ExecuteMsg;

pub fn transfer_callback(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    adapter: Hub,
    callback: IbcCallbackMsg,
) -> HubResult {
    // We burn the token that was successfully transfered (if so)

    let msg = match callback.result {
        CallbackResult::Execute {
            initiator_msg: _,
            result,
        } => {
            result.map_err(HubError::Transfer)?;

            let msg: HubIbcCallbackMsg = from_json(callback.msg.ok_or(HubError::Transfer(
                "There needs to be a message on callback".to_string(),
            ))?)?;

            let token_id = match msg {
                HubIbcCallbackMsg::BurnToken { token_id } => token_id,
            };

            let burn_msg = wasm_execute(
                NFT.load(deps.storage)?,
                &ExecuteMsg::Burn { token_id },
                vec![],
            )?;
            Ok(burn_msg)
        }
        CallbackResult::FatalError(error) => Err(HubError::Transfer(error)),
        _ => unreachable!(),
    }?;

    Ok(adapter.response("burn-token").add_message(msg))
}
