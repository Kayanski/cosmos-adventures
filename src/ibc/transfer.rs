use abstract_sdk::AbstractResponse;
use cosmwasm_std::{from_json, wasm_execute, Binary, DepsMut, Env, MessageInfo};
use polytone::callbacks::Callback;

use crate::{
    contract::{App, AppResult},
    error::AppError,
    msg::IbcCallbackMsg,
    state::NFT,
};
use cw721_metadata_onchain::ExecuteMsg;

pub fn transfer_callback(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    adapter: App,
    _id: String,
    message: Option<Binary>,
    callback: Callback,
) -> AppResult {
    // We burn the token that was successfully transfered (if so)

    let msg = match callback {
        Callback::Execute(execute_response) => {
            execute_response.map_err(AppError::Transfer)?;

            let msg: IbcCallbackMsg = from_json(message.ok_or(AppError::Transfer(
                "There needs to be a message on callback".to_string(),
            ))?)?;

            let token_id = match msg {
                IbcCallbackMsg::BurnToken { token_id } => token_id,
            };

            let burn_msg = wasm_execute(
                NFT.load(deps.storage)?,
                &ExecuteMsg::Burn { token_id },
                vec![],
            )?;
            Ok(burn_msg)
        }
        Callback::FatalError(error) => Err(AppError::Transfer(error)),
        _ => unreachable!(),
    }?;

    Ok(adapter.response("burn-token").add_message(msg))
}
