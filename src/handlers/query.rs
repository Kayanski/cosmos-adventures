use crate::contract::{App, AppResult};
use crate::msg::{AppQueryMsg, ConfigResponse};
use crate::state::{ACCOUNT, CONFIG, NFT};
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult};

pub fn query_handler(deps: Deps, _env: Env, _app: &App, msg: AppQueryMsg) -> AppResult<Binary> {
    match msg {
        AppQueryMsg::Config {} => to_json_binary(&query_config(deps)?),
    }
    .map_err(Into::into)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        nft: NFT.load(deps.storage)?.to_string(),
        account: ACCOUNT.load(deps.storage)?.account_id,
        next_token_id: config.next_token_id,
    })
}
