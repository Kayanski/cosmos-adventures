use cosmos_adventures_hub::contract::HUB_ID;
use cosmos_adventures_hub::CosmosAdventuresHub;

use cw_orch::prelude::*;

/// This is the raw way to access the cw-orchestrator logic.
/// I.e. this does not use the AbstractClient.
#[test]
fn successful_wasm() {
    // Create a sender
    let sender = Addr::unchecked("sender");
    // Create the mock
    let mock = Mock::new(sender);

    // Construct the counter interface
    let contract = CosmosAdventuresHub::new(HUB_ID, mock);
    // Panics if no path to a .wasm file is found
    contract.wasm();
}
