use crate::contract::{App, AppResult};
use crate::msg::AppInstantiateMsg;
use crate::replies::ACCOUNT_CREATION_REPLY;
use crate::state::{Config, CONFIG, NFT};
use abstract_core::objects::gov_type::GovernanceDetails;
use abstract_core::objects::module::{ModuleInfo, ModuleVersion};
use abstract_core::{account_factory, ACCOUNT_FACTORY};
use abstract_sdk::ModuleRegistryInterface;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, wasm_execute, Binary, CodeInfoResponse, DepsMut, Env,
    MessageInfo, QueryRequest, Response, SubMsg, WasmMsg, WasmQuery,
};
use cw721_base::InstantiateMsg;

pub fn instantiate_handler(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    adapter: App,
    msg: AppInstantiateMsg,
) -> AppResult {
    // We create the cw721 contract that will hold our NFTs
    let config: Config = Config {
        admin_account: msg.admin_account,
        next_token_id: 0,
        lost_token_uri: msg.lost_token_uri,
        lost_metadata: msg.lost_metadata,
    };

    // In instantiation, the adapter needs to create a new account to communicate with other chains
    let account_factory_contract = match adapter
        .module_registry(deps.as_ref())?
        .query_module(ModuleInfo::from_id(ACCOUNT_FACTORY, ModuleVersion::Latest)?)?
        .reference
    {
        abstract_core::objects::module_reference::ModuleReference::Native(addr) => addr,
        _ => unreachable!("Account factory is a native address"),
    };

    let create_account_msg = wasm_execute(
        account_factory_contract,
        &account_factory::ExecuteMsg::CreateAccount {
            governance: GovernanceDetails::Monarchy {
                monarch: env.contract.address.to_string(),
            },
            name: "Cosmos Adventures Hub".to_string(),
            base_asset: None,
            description: None,
            link: None,
            account_id: None,
            install_modules: vec![],
            namespace: None,
        },
        vec![],
    )?;

    // We also need to create the NFT contract that will host everything
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
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            create_account_msg,
            ACCOUNT_CREATION_REPLY,
        ))
        .add_message(nft_instantiation_msg))
}
