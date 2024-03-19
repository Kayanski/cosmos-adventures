use crate::ibc::{self, TRANSFER_CALLBACK};
use crate::msg::HubMigrateMsg;
use crate::{
    error::HubError,
    handlers,
    msg::{HubExecuteMsg, HubInstantiateMsg, HubQueryMsg},
};
use abstract_adapter::AdapterContract;
use cosmwasm_std::Response;

/// The version of your app
pub const HUB_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Namespace
pub const NAMESPACE: &str = "cosmos-adventures";
/// The id of the app
pub const HUB_ID: &str = "cosmos-adventures:hub";

/// The type of the result returned by your app's entry points.
pub type HubResult<T = Response> = Result<T, HubError>;

/// The type of the app that is used to build your app and access the Abstract SDK features.
pub type Hub =
    AdapterContract<HubError, HubInstantiateMsg, HubExecuteMsg, HubQueryMsg, HubMigrateMsg>;

const HUB: Hub = Hub::new(HUB_ID, HUB_VERSION, None)
    .with_instantiate(handlers::instantiate_handler)
    .with_execute(handlers::execute_handler)
    .with_query(handlers::query_handler)
    .with_replies(&[])
    .with_ibc_callbacks(&[(TRANSFER_CALLBACK, ibc::transfer::transfer_callback)])
    .with_module_ibc(ibc::module_ibc::receive_module_ibc);

// Export handlers
#[cfg(feature = "export")]
abstract_adapter::export_endpoints!(HUB, Hub);

#[cfg(feature = "interface")]
pub mod interface {
    use crate::msg::HubInstantiateMsg;
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use abstract_interface::AdapterDeployer;
    use abstract_interface::RegisteredModule;
    use abstract_sdk::features::ModuleIdentification;
    use cosmwasm_std::Empty;
    use cw_orch::contract::Contract;
    use cw_orch::interface;
    use cw_orch::prelude::*;

    use super::HUB;

    #[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
    pub struct CosmosAdventuresHub<Chain>;

    // Implement deployer trait
    impl<Chain: CwEnv> AdapterDeployer<Chain, HubInstantiateMsg> for CosmosAdventuresHub<Chain> {}

    impl<Chain: CwEnv> Uploadable for CosmosAdventuresHub<Chain> {
        fn wrapper(&self) -> <Mock as TxHandler>::ContractSource {
            Box::new(
                ContractWrapper::new_with_empty(
                    crate::contract::execute,
                    crate::contract::instantiate,
                    crate::contract::query,
                )
                .with_reply(crate::contract::reply),
            )
        }
        fn wasm(&self) -> WasmPath {
            artifacts_dir_from_workspace!()
                .find_wasm_path("cosmos-adventures-hub")
                .unwrap()
        }
    }

    impl<Chain: CwEnv> RegisteredModule for CosmosAdventuresHub<Chain> {
        type InitMsg = Empty;

        fn module_id<'a>() -> &'a str {
            HUB.module_id()
        }

        fn module_version<'a>() -> &'a str {
            HUB.version()
        }
    }

    impl<Chain: CwEnv> From<Contract<Chain>> for CosmosAdventuresHub<Chain> {
        fn from(contract: Contract<Chain>) -> Self {
            Self(contract)
        }
    }

    impl<Chain: cw_orch::environment::CwEnv> abstract_interface::DependencyCreation
        for CosmosAdventuresHub<Chain>
    {
        type DependenciesConfig = cosmwasm_std::Empty;

        fn dependency_install_configs(
            _configuration: Self::DependenciesConfig,
        ) -> Result<
            Vec<abstract_core::manager::ModuleInstallConfig>,
            abstract_interface::AbstractInterfaceError,
        > {
            Ok(vec![])
        }
    }
}
