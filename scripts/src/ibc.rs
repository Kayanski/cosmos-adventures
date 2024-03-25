use abstract_cw_orch_polytone::Polytone;
use abstract_interchain_tests::setup::ibc_connect_polytone_and_abstract;
use abstract_interface::Abstract;
use anyhow::Result as AnyResult;
use cw_orch::contract::Deploy;
use cw_orch_interchain::interchain::{IbcQueryHandler, InterchainEnv};

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
    Polytone::deploy_on(origin_chain.clone(), None)?;
    Polytone::deploy_on(remote_chain.clone(), None)?;

    ibc_connect_polytone_and_abstract(interchain, origin_chain_id, remote_chain_id)?;

    Ok((abstr_origin, abstr_remote))
}
