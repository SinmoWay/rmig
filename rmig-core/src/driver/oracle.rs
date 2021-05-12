use r2d2_oracle::OracleConnectionManager;
use r2d2_oracle::r2d2::Pool;
use crate::driver::{DriverFactory, Driver, RmigEmptyResult};
use crate::configuration_properties::DatasourceProperties;
use crate::error::Error;
use log::debug;
use std::collections::{VecDeque, HashMap};
use crate::changelogs::{Query, Migration};
use async_trait::async_trait;
use std::borrow::Borrow;

#[derive(Clone, Debug)]
pub struct DatasourceOracle {
    pub name: String,
    pub pool: Box<Pool<OracleConnectionManager>>,
    pub schema_admin: String,
    pub separator: String,
}

impl DriverFactory<DatasourceOracle> for DatasourceOracle {
    fn new(props: &DatasourceProperties) -> DatasourceOracle {
        // TODO: Creat fn in mod
        // TODO: Create method from_str (impl trait)

        let url = props.full_url.as_ref().expect("Url for datasource is required.").as_str();
        let _url = url::Url::parse(&*url).map_err(|_e| Error::CreatingDatasourceError("Url is not valid. Check your configuration and url parameters.".to_string())).unwrap();

        let host = _url.host_str().expect("Not found hostname.").to_owned();
        let port = _url.port().expect("Port required").to_owned().to_string();

        let schema_admin = props.properties.as_ref().unwrap_or(HashMap::<String, String>::new().borrow()).get("SCHEMA_ADMIN").unwrap_or(&"".to_string()).to_string();

        let mut separator = "";
        if !schema_admin.is_empty() {
            separator = ".";
        }
        // TODO: Required?
        let password = _url.password().expect("Expected password").to_owned();
        let user = _url.username().to_owned();
        let path = _url.path().trim_start_matches('/');

        if path.is_empty() {
            panic!("Datasource name is required, for oracle connection.");
        }

        debug!("Creating datasource pool: {}. User: {}", &*host, &*user);
        let manager = OracleConnectionManager::new(
            &*user,
            &*password,
            format!("{}:{}/{}", host, port, path).as_str(),
        );

        let pool = Box::new(Pool::new(manager).expect("Error while creating datasource pool for oracle driver."));
        DatasourceOracle {
            name: host,
            pool,
            schema_admin,
            separator: separator.to_string(),
        }
    }
}

#[async_trait]
impl Driver for DatasourceOracle {
    fn validate_connection(&self) -> RmigEmptyResult {
        unimplemented!()
    }

    fn migrate(&self, query: VecDeque<&Query>) -> RmigEmptyResult {
        unimplemented!()
    }

    fn find_in_core_table(&self, name: String, hash: String) -> RmigEmptyResult {
        unimplemented!()
    }

    fn check_rmig_core_table(&self) -> RmigEmptyResult {
        unimplemented!()
    }

    fn create_rmig_core_table(&self) -> RmigEmptyResult {
        unimplemented!()
    }

    fn get_name(&self) -> &str {
        unimplemented!()
    }

    fn close(&self) {
        unimplemented!()
    }

    async fn lock(&self) -> RmigEmptyResult {
        unimplemented!()
    }

    async fn unlock(&self) -> RmigEmptyResult {
        unimplemented!()
    }

    async fn add_new_migration(&self, migration: Migration) -> RmigEmptyResult {
        Ok(())
    }
}

impl Drop for DatasourceOracle {
    fn drop(&mut self) {}
}