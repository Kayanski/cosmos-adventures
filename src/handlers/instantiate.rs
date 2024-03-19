use crate::contract::{Hub, HubResult};
use crate::msg::HubInstantiateMsg;
use crate::state::{Config, CONFIG, NFT};
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Binary, CodeInfoResponse, DepsMut, Env, MessageInfo,
    QueryRequest, Response, WasmMsg, WasmQuery,
};
use cw721_base::InstantiateMsg;

pub fn instantiate_handler(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _hub: Hub,
    msg: HubInstantiateMsg,
) -> HubResult {
    let config: Config = Config {
        next_token_id: 0,
        lost_token_uri: msg.lost_token_uri,
        lost_metadata: msg.lost_metadata,
    };

    // We need to create the NFT contract that will host everything locally
    let salt = b"nft_contract".to_vec();
    let nft_instantiation_msg = WasmMsg::Instantiate2 {
        admin: Some(env.contract.address.to_string()),
        code_id: msg.nft_code_id,
        label: "Cosmos Adventures NFT".to_string(),
        msg: to_json_binary(&InstantiateMsg {
            name: "Cosmos Adventurers".to_string(),
            symbol: "IBC-CA".to_string(),
            minter: env.contract.address.to_string(),
        })?,
        funds: vec![],
        salt: Binary(salt.clone()),
    };
    let code_id_info: CodeInfoResponse =
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::CodeInfo {
                code_id: msg.nft_code_id,
            }))?;
    let canon_nft = instantiate2_address(
        code_id_info.checksum.as_slice(),
        &deps.api.addr_canonicalize(env.contract.address.as_str())?,
        &salt,
    )?;
    NFT.save(deps.storage, &deps.api.addr_humanize(&canon_nft)?)?;

    CONFIG.save(deps.storage, &config)?;

    // Example instantiation that doesn't do anything
    Ok(Response::new().add_message(nft_instantiation_msg))
}
