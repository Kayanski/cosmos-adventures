use abstract_core::{
    ibc_client::{ExecuteMsgFns as _, QueryMsgFns as _},
    ibc_host::ExecuteMsgFns as _,
    objects::chain_name::ChainName,
};
use abstract_cw_orch_polytone::Polytone;
use abstract_interface::Abstract;
use abstract_polytone::handshake::POLYTONE_VERSION;
use anyhow::Result as AnyResult;
use cw_orch::prelude::*;
use cw_orch_interchain::interchain::{IbcQueryHandler, InterchainEnv, InterchainError};

/// This is only used for testing and shouldn't be used in production
fn abstract_ibc_connection_with<Chain: IbcQueryHandler, IBC: InterchainEnv<Chain>>(
    abstr: &Abstract<Chain>,
    interchain: &IBC,
    dest: &Abstract<Chain>,
    polytone_src: &Polytone<Chain>,
) -> Result<(), InterchainError> {
    // First we register client and host respectively
    let chain1_id = abstr.ibc.client.get_chain().chain_id();
    let chain1_name = ChainName::from_chain_id(&chain1_id);

    let chain2_id = dest.ibc.client.get_chain().chain_id();
    let chain2_name = ChainName::from_chain_id(&chain2_id);

    // First, we register the host with the client.
    // We register the polytone note with it because they are linked
    // This triggers an IBC message that is used to get back the proxy address
    let proxy_tx_result = abstr.ibc.client.register_infrastructure(
        chain2_name.to_string(),
        dest.ibc.host.address()?.to_string(),
        polytone_src.note.address()?.to_string(),
    )?;
    // We make sure the IBC execution is done so that the proxy address is saved inside the Abstract contract
    interchain.wait_ibc(&chain1_id, proxy_tx_result).unwrap();

    // Finally, we get the proxy address and register the proxy with the ibc host for the dest chain
    let proxy_address = abstr.ibc.client.host(chain2_name.to_string())?;

    dest.ibc.host.register_chain_proxy(
        chain1_name.to_string(),
        proxy_address.remote_polytone_proxy.unwrap(),
    )?;

    Ok(())
}

pub fn ibc_abstract_setup<Chain: IbcQueryHandler, IBC: InterchainEnv<Chain>>(
    interchain: &IBC,
    origin_chain_id: &str,
    remote_chain_id: &str,
) -> AnyResult<(Abstract<Chain>, Abstract<Chain>)> {
    let origin_chain = interchain.chain(origin_chain_id).unwrap();
    let remote_chain = interchain.chain(remote_chain_id).unwrap();

    // Deploying abstract and the IBC abstract logic
    let abstr_origin = Abstract::load_from(origin_chain.clone())?;
    let abstr_remote = Abstract::load_from(remote_chain.clone())?;

    // Deploying polytone on both chains
    let origin_polytone = Polytone::deploy_on(origin_chain.clone(), None)?;
    let remote_polytone = Polytone::deploy_on(remote_chain.clone(), None)?;

    // Creating a connection between 2 polytone deployments
    interchain.create_contract_channel(
        &origin_polytone.note,
        &remote_polytone.voice,
        POLYTONE_VERSION,
    )?;

    // Create the connection between client and host
    abstract_ibc_connection_with(&abstr_origin, interchain, &abstr_remote, &origin_polytone)?;

    Ok((abstr_origin, abstr_remote))
}
