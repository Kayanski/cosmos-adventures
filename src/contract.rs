use crate::ibc::{self, TRANSFER_CALLBACK};
use crate::msg::AppMigrateMsg;
use crate::{
    error::AppError,
    handlers,
    msg::{AppExecuteMsg, AppInstantiateMsg, AppQueryMsg},
    replies::{self, ACCOUNT_CREATION_REPLY},
};
use abstract_adapter::AdapterContract;
use cosmwasm_std::Response;

/// The version of your app
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Namespace
pub const NAMESPACE: &str = "comsos-adventures";
/// The id of the app
pub const APP_ID: &str = "cosmos-adventures:hub";

/// The type of the result returned by your app's entry points.
pub type AppResult<T = Response> = Result<T, AppError>;

/// The type of the app that is used to build your app and access the Abstract SDK features.
pub type App =
    AdapterContract<AppError, AppInstantiateMsg, AppExecuteMsg, AppQueryMsg, AppMigrateMsg>;

const APP: App = App::new(APP_ID, APP_VERSION, None)
    .with_instantiate(handlers::instantiate_handler)
    .with_execute(handlers::execute_handler)
    .with_query(handlers::query_handler)
    .with_replies(&[(ACCOUNT_CREATION_REPLY, replies::account_creation_reply)])
    .with_ibc_callbacks(&[(TRANSFER_CALLBACK, ibc::transfer::transfer_callback)]);

// Export handlers
#[cfg(feature = "export")]
abstract_adapter::export_endpoints!(APP, App);

#[cfg(feature = "interface")]
pub mod interface {
    use crate::msg::AppInstantiateMsg;
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use abstract_interface::AdapterDeployer;
    use abstract_interface::RegisteredModule;
    use abstract_sdk::features::ModuleIdentification;
    use cosmwasm_std::Empty;
    use cw_orch::contract::Contract;
    use cw_orch::interface;
    use cw_orch::prelude::*;

    use super::APP;

    #[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
    pub struct CosmosAdventuresHub<Chain>;

    // Implement deployer trait
    impl<Chain: CwEnv> AdapterDeployer<Chain, AppInstantiateMsg> for CosmosAdventuresHub<Chain> {}

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
            APP.module_id()
        }

        fn module_version<'a>() -> &'a str {
            APP.version()
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
