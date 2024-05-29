use abstract_adapter::std::objects::chain_name::ChainName;
use cosmwasm_std::{Deps, DepsMut, Env};

use crate::{contract::HubResult, state::CONFIG};

pub fn next_token_id_mut(deps: DepsMut, env: Env) -> HubResult<String> {
    let next_token_id = next_token_id(deps.as_ref(), env)?;

    let mut config = CONFIG.load(deps.storage)?;
    config.next_token_id += 1;
    CONFIG.save(deps.storage, &config)?;

    Ok(next_token_id)
}

pub fn next_token_id(deps: Deps, env: Env) -> HubResult<String> {
    let config = CONFIG.load(deps.storage)?;
    let chain_name = ChainName::from_chain_id(&env.block.chain_id);

    let next_token_id = format!("{}>{}", chain_name.to_string(), config.next_token_id);

    Ok(next_token_id)
}
