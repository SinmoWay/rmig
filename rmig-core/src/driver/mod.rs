use crate::changelogs::{Migration, Query};
use crate::configuration_properties::DatasourceProperties;
use crate::error::Error;
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};

#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "ora")]
pub mod oracle;
#[cfg(feature = "postgres")]
pub mod postgres;

use crate::enum_str;

pub type RmigEmptyResult = anyhow::Result<(), Error>;

#[async_trait]
pub trait Driver {
    /// Validation connection per 5 request's.
    fn validate_connection(&self) -> RmigEmptyResult;

    /// Run migration query, including parameters
    fn migrate(&self, query: VecDeque<&Query>) -> RmigEmptyResult;

    /// Find row in core table, if exists return empty OK
    /// If row is not found, or found but hash is changed, return Err [RowError]
    fn find_in_core_table(&self, name: String, hash: String) -> RmigEmptyResult;

    /// Find rmig table. If core table exists, return OK(), if core table does not exists, return Err()
    fn check_rmig_core_table(&self) -> RmigEmptyResult;

    /// Create rmig table for changelogs.
    fn create_rmig_core_table(&self) -> RmigEmptyResult;

    /// Getting driver name or hostname, or name.
    fn get_name(&self) -> &str;

    /// Close connection on destroy ref
    fn close(&self);

    /// Locking current DB for migration
    /// 1. Try lock, if lock acquired = true, loop until acquired = false
    /// 2. Locking current host.
    async fn lock(&self) -> RmigEmptyResult;

    /// Unlock current DB for migration
    /// Delete row.
    async fn unlock(&self) -> RmigEmptyResult;

    async fn add_new_migration(&self, migration: Migration) -> RmigEmptyResult;
}

pub trait DriverFactory<T: Clone + Drop + Driver + Sized> {
    fn new(props: &DatasourceProperties) -> T;
}

/// Factory component for creating Datasource's and methods for migration.
pub struct DatasourceFactory {}

impl DatasourceFactory {
    pub fn new(props: &DatasourceProperties) -> anyhow::Result<Box<dyn Driver>, Error> {
        let url = props
            .full_url
            .as_ref()
            .expect("Url for datasource is required.")
            .to_string();

        #[cfg(feature = "postgres")]
        if url.trim().starts_with("postgres") {
            return Ok(Box::new(crate::driver::postgres::DatasourcePostgres::new(
                props,
            )));
        }

        #[cfg(feature = "ora")]
        if url.trim().starts_with("oracle") {
            // return Ok(Box::new(crate::driver::oracle::DatasourceOracle::new(props)));
            panic!("Oracle driver no impl.");
        }

        #[cfg(feature = "mysql")]
        if url.trim().starts_with("mysql") {
            panic!("Mysql driver no impl.");
        }

        let _url = url::Url::parse(&*url)
            .expect("Error while parsing url. Please verify and try again.")
            .host_str()
            .expect("Url is not valid. Empty host.")
            .to_string();
        Err(Error::ConnectionValidationError(
            format!("Driver by url {} not found.", &*_url).to_owned(),
        ))
    }
}

fn generate_lock(db_name: String) -> i64 {
    let mut x = crc32fast::Hasher::new();
    x.update(db_name.as_bytes());
    x.finalize() as i64
}

struct DatasourceWrapper {
    properties: Box<DatasourceProperties>,
}

impl DatasourceWrapper {
    pub fn new(properties: Box<DatasourceProperties>) -> Self {
        DatasourceWrapper { properties }
    }

    pub fn get_url(&self) -> &str {
        self.properties
            .full_url
            .as_ref()
            .expect("Url for datasource is required.")
            .as_str()
    }

    pub fn get_name(&self) -> String {
        url::Url::parse(self.get_url())
            .as_ref()
            .map_err(|_e| {
                Error::CreatingDatasourceError(
                    "Url is not valid. Check your configuration and url parameters.".to_string(),
                )
            })
            .unwrap()
            .host_str()
            .expect("Not found hostname.")
            .to_owned()
    }

    pub fn get_schema_admin(&self) -> String {
        self.properties
            .properties
            .as_ref()
            .unwrap_or(&HashMap::<String, String>::new())
            .get("SCHEMA_ADMIN")
            .unwrap_or(&"".to_string())
            .to_string()
    }

    pub fn get_separator(&self) -> String {
        let schema_admin = self.get_schema_admin();
        let mut separator = "";
        if !schema_admin.is_empty() {
            separator = ".";
        }
        separator.to_owned()
    }
}

enum_str! {
    pub enum DriverOptions {
        MaxPoolSize = 0x000000,
        MinPoolSize = 0x000001,
        ConnectionTimeout = 0x000002,
        MaxLifetime = 0x000003,
        IldeTimeout = 0x000004,
        AfterConnect = 0x000005,
        AfterConnectScript = 0x000006,
    }
}

#[cfg(test)]
mod test_local {
    use crate::driver::DriverOptions;

    #[test]
    pub fn parameter_name_eq() -> anyhow::Result<()> {
        assert_eq!("MaxPoolSize", DriverOptions::MaxPoolSize.name());
        assert_eq!("MinPoolSize", DriverOptions::MinPoolSize.name());
        assert_eq!("ConnectionTimeout", DriverOptions::ConnectionTimeout.name());
        assert_eq!("MaxLifetime", DriverOptions::MaxLifetime.name());
        assert_eq!("IldeTimeout", DriverOptions::IldeTimeout.name());
        assert_eq!("AfterConnect", DriverOptions::AfterConnect.name());
        assert_eq!(
            "AfterConnectScript",
            DriverOptions::AfterConnectScript.name()
        );
        Ok(())
    }
}