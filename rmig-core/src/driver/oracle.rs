use r2d2_oracle::OracleConnectionManager;
use r2d2_oracle::r2d2::Pool;
use crate::driver::{DriverFactory, Driver, RmigEmptyResult, DatasourceWrapper};
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
        let wrapper = DatasourceWrapper::new(Box::new(props.to_owned()));
        let _url = url::Url::parse(&*wrapper.get_url())
            .map_err(|_e| Error::CreatingDatasourceError("Url is not valid. Check your configuration and url parameters.".to_string()))
            .unwrap();

        let host = _url.host_str().expect("Not found hostname.").to_owned();
        let port = _url.port().expect("Port required").to_owned().to_string();

        let password = _url.password().unwrap_or_default().to_owned();
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
            schema_admin: wrapper.get_schema_admin(),
            separator: wrapper.get_separator(),
        }
    }
}

#[async_trait]
impl Driver for DatasourceOracle {
    fn validate_connection(&self) -> RmigEmptyResult {
        self.pool.get().expect("Error while getting connection").ping().map_err(|e| Error::ConnectionValidationError(format!("{:?}", e)))
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