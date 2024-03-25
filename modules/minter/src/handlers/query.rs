use crate::contract::{Minter, MinterResult};
use crate::msg::{ConfigResponse, MinterQueryMsg};
use crate::state::CONFIG;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult};

pub fn query_handler(
    deps: Deps,
    _env: Env,
    _app: &Minter,
    msg: MinterQueryMsg,
) -> MinterResult<Binary> {
    match msg {
        MinterQueryMsg::Config {} => to_json_binary(&query_config(deps)?),
    }
    .map_err(Into::into)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        metadata_base: config.metadata_base,
        token_uri_base: config.token_uri_base,
        admin_account: config.admin_account,
        mint_limit: config.mint_limit,
        mint_cost: config.mint_cost,
    })
}
