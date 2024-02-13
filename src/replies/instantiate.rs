use crate::{
    contract::{App, AppResult},
    state::{Account, ACCOUNT},
};
use abstract_core::{manager, objects::AccountId, ABSTRACT_EVENT_TYPE};
use abstract_sdk::{features::AbstractRegistryAccess, AbstractResponse};
use cosmwasm_std::{wasm_execute, DepsMut, Env, Reply, StdError};

pub fn account_creation_reply(deps: DepsMut, _env: Env, adapter: App, reply: Reply) -> AppResult {
    // Parse event and get account proxy
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(response) => {
            // We find the created account in instantiation

            let abstract_event = response
                .events
                .into_iter()
                .find(|e| e.ty == ABSTRACT_EVENT_TYPE)
                .ok_or(StdError::generic_err("Abstract event type not found"))?
                .attributes;

            let acc_seq = abstract_event
                .iter()
                .find(|e| e.key == "account_sequence")
                .ok_or(StdError::generic_err("Account sequence not found"))?
                .value
                .clone();
            let trace = abstract_event
                .iter()
                .find(|e| e.key == "trace")
                .ok_or(StdError::generic_err("Account trace not found"))?
                .value
                .clone();

            let account_id = AccountId::new(
                acc_seq.parse().unwrap(),
                abstract_core::objects::account::AccountTrace::try_from(trace.as_str())?,
            )?;

            ACCOUNT.save(
                deps.storage,
                &Account {
                    account_id: account_id.clone(),
                },
            )?;
            let ibc_enable_msg = wasm_execute(
                adapter
                    .abstract_registry(deps.as_ref())?
                    .account_base(&account_id, &deps.querier)?
                    .manager,
                &manager::ExecuteMsg::UpdateSettings {
                    ibc_enabled: Some(true),
                },
                vec![],
            )?;
            Ok(adapter
                .response("instantiate_reply")
                .add_message(ibc_enable_msg))
        }
        cosmwasm_std::SubMsgResult::Err(_) => unreachable!(),
    }
}
