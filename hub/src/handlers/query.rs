use crate::contract::{Hub, HubResult};
use crate::msg::{ConfigResponse, HubQueryMsg};
use crate::state::{CONFIG, NFT};
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult};

pub fn query_handler(deps: Deps, _env: Env, _app: &Hub, msg: HubQueryMsg) -> HubResult<Binary> {
    match msg {
        HubQueryMsg::Config {} => to_json_binary(&query_config(deps)?),
    }
    .map_err(Into::into)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        nft: NFT.load(deps.storage)?.to_string(),
        next_token_id: config.next_token_id,
    })
}
