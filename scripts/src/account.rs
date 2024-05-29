use abstract_ibc_host::endpoints::packet::client_to_host_account_id;
use abstract_interface::Abstract;
use abstract_sdk::std::objects::chain_name::ChainName;
use abstract_sdk::std::objects::module::ModuleVersion;
use abstract_sdk::std::objects::{salt::generate_instantiate_salt, AccountId};
use abstract_sdk::std::PROXY;
use cw_orch::contract::Deploy;
use cw_orch::environment::CwEnv;
use cw_orch::prelude::ContractInstance;
use cw_orch::prelude::WasmQuerier;

pub fn get_proxy_address<Chain: CwEnv>(
    client_chain: &Chain,
    target_chain: &Chain,
    client_id: AccountId,
) -> anyhow::Result<String> {
    let client_chain_name = ChainName::from_chain_id(&client_chain.block_info().unwrap().chain_id);
    let target_id = client_to_host_account_id(client_chain_name, client_id);
    let salt = generate_instantiate_salt(&target_id);

    let abs = Abstract::load_from(target_chain.clone())?;
    let proxy_code_id = abs
        .version_control
        .get_module_code_id(PROXY, ModuleVersion::Latest)?;

    Ok(target_chain
        .wasm_querier()
        .instantiate2_addr(
            proxy_code_id,
            abs.account_factory.address()?.to_string(),
            salt,
        )
        .unwrap())
}

// // Get code_ids
// let (proxy_code_id, manager_code_id) = if let (
//     ModuleReference::AccountBase(proxy_code_id),
//     ModuleReference::AccountBase(manager_code_id),
// ) = (
//     proxy_module.reference.clone(),
//     manager_module.reference.clone(),
// ) {
//     (proxy_code_id, manager_code_id)
// } else {
//     return Err(AccountFactoryError::WrongModuleKind(
//         proxy_module.info.to_string(),
//         "account_base".to_string(),
//     ));
// };

// // Get checksums
// let proxy_checksum = deps.querier.query_wasm_code_info(proxy_code_id)?.checksum;
// let manager_checksum = deps.querier.query_wasm_code_info(manager_code_id)?.checksum;
