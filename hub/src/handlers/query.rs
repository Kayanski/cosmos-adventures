use crate::contract::{Hub, HubResult};
use crate::helpers::next_token_id;
use crate::msg::{ConfigResponse, HubQueryMsg, NextTokenIdResponse};
use crate::state::{CONFIG, NFT};
use cosmwasm_std::{to_json_binary, Binary, Deps, Env};

pub fn query_handler(deps: Deps, env: Env, _app: &Hub, msg: HubQueryMsg) -> HubResult<Binary> {
    match msg {
        HubQueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        HubQueryMsg::NextTokenId {} => to_json_binary(&query_next_token_id(deps, env)?),
    }
    .map_err(Into::into)
}

fn query_config(deps: Deps) -> HubResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        nft: NFT.load(deps.storage)?.to_string(),
        next_token_id: config.next_token_id,
    })
}

fn query_next_token_id(deps: Deps, env: Env) -> HubResult<NextTokenIdResponse> {
    Ok(NextTokenIdResponse {
        next_token_id: next_token_id(deps, env)?,
    })
}
