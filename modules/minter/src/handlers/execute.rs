use abstract_adapter::std::ibc_client::{
    InstalledModuleIdentification, ListIbcInfrastructureResponse,
};
use abstract_adapter::std::objects::module::ModuleInfo;
use abstract_adapter::std::{ibc_client, IBC_CLIENT};
use abstract_sdk::{
    AbstractResponse, AccountVerification, Execution, ExecutorMsg, ModuleInterface,
    TransferInterface,
};
use cosmwasm_std::{to_json_binary, wasm_execute, Deps, DepsMut, Env, MessageInfo};

use crate::contract::{Minter, MinterResult};
use crate::error::MinterError;
use crate::msg::{MinterExecuteMsg, MinterIbcMsg};
use crate::state::{CONFIG, CURRENT_MINTED_AMOUNT};
use abstract_sdk::features::{AccountIdentification, ModuleIdentification};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: Minter,
    msg: MinterExecuteMsg,
) -> MinterResult {
    match msg {
        MinterExecuteMsg::Mint { send_back } => mint(deps, info, env, adapter, send_back),
    }
}

fn mint(
    mut deps: DepsMut,
    _info: MessageInfo,
    _env: Env,
    adapter: Minter,
    send_back: bool,
) -> MinterResult {
    // We make sure this account is a remote account, with an associated trace
    let account = adapter.account_id(deps.as_ref())?;

    // We make sure the account can still mint new tokens on this target chain
    assert_mint_limit(deps.branch(), &adapter)?;

    // We make the user pay some tokens to mint
    let payment_msg = payment(deps.as_ref(), &adapter)?;

    let ibc_client_addr = adapter.modules(deps.as_ref()).module_address(IBC_CLIENT)?;
    let all_chains: ListIbcInfrastructureResponse = deps.querier.query_wasm_smart(
        &ibc_client_addr,
        &ibc_client::QueryMsg::ListIbcInfrastructures {},
    )?;

    let recipient_chain =
        all_chains.counterparts[account.seq() as usize % all_chains.counterparts.len()].clone();

    // We send an IBC message for the mint to happen on the other chain
    let current_module_info = ModuleInfo::from_id(adapter.module_id(), adapter.version().into())?;
    let ibc_msg = ibc_client::ExecuteMsg::ModuleIbcAction {
        host_chain: recipient_chain.0.to_string(),
        target_module: current_module_info,
        msg: to_json_binary(&MinterIbcMsg::IbcMint {
            local_account_id: account,
            send_back,
        })?,
        callback_info: None,
    };

    let mint_msg = wasm_execute(ibc_client_addr, &ibc_msg, vec![])?;

    Ok(adapter
        .response("mint-lost-nft")
        .add_message(mint_msg)
        .add_message(payment_msg))
}

fn payment(deps: Deps, adapter: &Minter) -> MinterResult<ExecutorMsg> {
    let config = CONFIG.load(deps.storage)?;
    let admin_account_base = adapter
        .account_registry(deps)?
        .account_base(&config.admin_account)?;
    let payment_msg = adapter
        .bank(deps)
        .transfer(vec![config.mint_cost], &admin_account_base.proxy)?;
    Ok(adapter.executor(deps).execute(vec![payment_msg])?)
}

fn assert_mint_limit(deps: DepsMut, adapter: &Minter) -> MinterResult<()> {
    let config = CONFIG.load(deps.storage)?;
    // We make sure the account can still mint new tokens
    let account_id = adapter.account_id(deps.as_ref())?;
    let current_minted_amount = CURRENT_MINTED_AMOUNT
        .may_load(deps.storage, &account_id)?
        .unwrap_or(0);

    if current_minted_amount >= config.mint_limit {
        return Err(MinterError::TooMuchMinted(config.mint_limit));
    }

    CURRENT_MINTED_AMOUNT.save(deps.storage, &account_id, &(current_minted_amount + 1))?;

    Ok(())
}
