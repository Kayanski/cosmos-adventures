use abstract_core::objects::chain_name::ChainName;
use cosmwasm_std::{DepsMut, Env};

use crate::{contract::HubResult, state::CONFIG};

pub fn next_token_id(deps: DepsMut, env: Env) -> HubResult<String> {
    let mut config = CONFIG.load(deps.storage)?;
    let chain_name = ChainName::from_chain_id(&env.block.chain_id);

    let next_token_id = format!("{}>{}", chain_name.to_string(), config.next_token_id);
    config.next_token_id += 1;
    CONFIG.save(deps.storage, &config)?;

    Ok(next_token_id)
}
