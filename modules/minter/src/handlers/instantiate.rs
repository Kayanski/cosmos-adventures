use crate::contract::{Minter, MinterResult};
use crate::msg::MinterInstantiateMsg;
use crate::state::{Config, CONFIG};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

pub fn instantiate_handler(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _hub: Minter,
    msg: MinterInstantiateMsg,
) -> MinterResult {
    let config = Config {
        admin_account: msg.admin_account,
        metadata_base: msg.metadata_base,
        token_uri_base: msg.token_uri_base,
        mint_limit: msg.mint_limit,
        mint_cost: msg.mint_cost,
    };

    CONFIG.save(deps.storage, &config)?;

    // Example instantiation that doesn't do anything
    Ok(Response::new().add_attribute("instantiate", "minter-adapter"))
}
