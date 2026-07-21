//! PostgreSQL data-layer wiring for finch.
//!
//! Wire this into each surface host (cli/src/main.rs, tui/src/main.rs,
//! desktop/src-tauri/src/main.rs):
//!
//! ```ignore
//! use nest_data::DataModule;
//! use nest_data_postgres::PostgresDataModule;
//! use finch_core::data_postgres::FinchDataModule;
//!
//! CliApp::new("...")
//!     .module(DataModule)
//!     .module(PostgresDataModule::from_env("DATABASE_URL")?)
//!     .module(FinchDataModule)
//!     .run();
//! ```

use nest_core::{AppBuilder, Module, ModuleId, NestResult};
use nest_data_postgres::{PostgresConnection, POSTGRES_DATA_MODULE_ID};

use crate::settings::{settings_migration, SettingsRepository};

/// App-specific data module. Depends on the PostgreSQL provider.
pub struct FinchDataModule;

impl Module for FinchDataModule {
    fn id(&self) -> ModuleId {
        ModuleId("finch-data")
    }

    fn dependencies(&self) -> &'static [ModuleId] {
        &[POSTGRES_DATA_MODULE_ID]
    }

    fn configure(&self, app: &mut AppBuilder) -> NestResult<()> {
        let conn = app.service_mut::<PostgresConnection>()?.clone();
        app.register_service(SettingsRepository::new(conn))
    }
}

/// Returns the migrations required by Finch repositories.
pub fn finch_migrations() -> Vec<Box<dyn nest_data::Migration>> {
    vec![Box::new(settings_migration())]
}
