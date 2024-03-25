use crate::ibc;
use crate::msg::MinterMigrateMsg;
use crate::{
    error::MinterError,
    handlers,
    msg::{MinterExecuteMsg, MinterInstantiateMsg, MinterQueryMsg},
};
use abstract_adapter::AdapterContract;
use cosmwasm_std::Response;

/// The version of your app
pub const MINTER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The id of the app
pub const MINTER_ID: &str = "cosmos-adventures:cross-chain-mint";

/// The type of the result returned by your app's entry points.
pub type MinterResult<T = Response> = Result<T, MinterError>;

/// The type of the app that is used to build your app and access the Abstract SDK features.
pub type Minter = AdapterContract<
    MinterError,
    MinterInstantiateMsg,
    MinterExecuteMsg,
    MinterQueryMsg,
    MinterMigrateMsg,
>;

const MINTER: Minter = Minter::new(MINTER_ID, MINTER_VERSION, None)
    .with_instantiate(handlers::instantiate_handler)
    .with_execute(handlers::execute_handler)
    .with_query(handlers::query_handler)
    .with_module_ibc(ibc::mint::receive_module_ibc)
    .with_replies(&[])
    .with_ibc_callbacks(&[]);

// Export handlers
#[cfg(feature = "export")]
abstract_adapter::export_endpoints!(MINTER, Minter);

#[cfg(feature = "interface")]
pub mod interface {
    use crate::msg::MinterInstantiateMsg;
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use abstract_interface::AdapterDeployer;
    use abstract_interface::RegisteredModule;
    use abstract_sdk::features::ModuleIdentification;
    use cosmwasm_std::Empty;
    use cw_orch::contract::Contract;
    use cw_orch::interface;
    use cw_orch::prelude::*;

    use super::MINTER;

    #[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
    pub struct CosmosAdventuresMinter<Chain>;

    // Implement deployer trait
    impl<Chain: CwEnv> AdapterDeployer<Chain, MinterInstantiateMsg> for CosmosAdventuresMinter<Chain> {}

    impl<Chain: CwEnv> Uploadable for CosmosAdventuresMinter<Chain> {
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

    impl<Chain: CwEnv> RegisteredModule for CosmosAdventuresMinter<Chain> {
        type InitMsg = Empty;

        fn module_id<'a>() -> &'a str {
            MINTER.module_id()
        }

        fn module_version<'a>() -> &'a str {
            MINTER.version()
        }
    }

    impl<Chain: CwEnv> From<Contract<Chain>> for CosmosAdventuresMinter<Chain> {
        fn from(contract: Contract<Chain>) -> Self {
            Self(contract)
        }
    }

    impl<Chain: cw_orch::environment::CwEnv> abstract_interface::DependencyCreation
        for CosmosAdventuresMinter<Chain>
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
