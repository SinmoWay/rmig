use crate::changelogs::{Migration, Query};
use crate::configuration_properties::DatasourceProperties;
use crate::driver::{DatasourceWrapper, Driver, DriverFactory, RmigEmptyResult};
use crate::error::Error;
use crate::tera_manager::TeraManager;
use async_trait::async_trait;
use log::{debug, error, info};
use r2d2_oracle::oracle::Statement;
use r2d2_oracle::r2d2::Pool;
use r2d2_oracle::OracleConnectionManager;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

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
            .map_err(|_e| {
                Error::CreatingDatasourceError(
                    "Url is not valid. Check your configuration and url parameters.".to_string(),
                )
            })
            .unwrap();

        let host = _url.host_str().expect("Not found hostname.").to_owned();
        let port = _url.port().expect("Port required").to_owned().to_string();

        let password = _url.password().unwrap_or_default().to_owned();
        let user = _url.username().to_owned();
        let path = _url.path().trim_start_matches('/');

        if path.is_empty() {
            panic!("Datasource name is required, for oracle connection.");
        }

        let manager = OracleConnectionManager::new(
            &*user,
            &*password,
            format!("{}:{}/{}", host, port, path).as_str(),
        );

        let pool = Box::new(
            Pool::new(manager).expect("Error while creating datasource pool for oracle driver."),
        );
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
        let conn = self.pool.get().expect("Error while getting connection");
        let rows = conn
            .query_as::<(i32)>("SELECT 1 FROM DUAL;", &[])
            .map_err(|e| Error::ConnectionValidationError(format!("{:?}", e)))?;

        for row in rows {
            let i = row.map_err(|e| Error::ConnectionValidationError(format!("{:?}", e)))?;

            if i != 1 {
                return Err(Error::RowError(format!(
                    "Connection error. SELECT 1 FROM DUAL, return unexpected result: {}",
                    i
                )));
            }
        }

        Ok(())
    }

    fn migrate(&self, query: VecDeque<&Query>) -> RmigEmptyResult {
        let mut conn = self.pool.get().expect("Error while getting connection");

        if conn.autocommit() {
            conn.set_autocommit(false)
        }

        let start = Instant::now();

        for q in query {
            debug!("Running sql: {}", &*q.query);
            // Execute, if error, rollback transaction
            match conn
                .execute(&*q.query, &[])
                .map_err(|e| Error::SQLError(format!("{:?}.\nSQL - {}", e, &*q.query)))
            {
                // Ignore normal statement
                Ok(_o) => Ok(()),
                // If query return error, rollback transaction
                Err(e) => {
                    error!("Query {}\nexited with error.", &*q.query);
                    conn.rollback()
                        .map_err(|e| {
                            Error::RowError(format!("{:?}. Transaction is not rollback.", e))
                        })
                        .unwrap();
                    Err(Error::RowError(format!("{:?}", e)))
                }
            }?;
        }

        conn.commit()
            .map_err(|e| Error::RowError(format!("{:?}. Transaction is not commit.", e)))?;

        let elapsed = start.elapsed();

        debug!("Success! Time elapsed: {:?}", elapsed);

        Ok(())
    }

    fn find_in_core_table(&self, name: String, hash: String) -> RmigEmptyResult {
        unimplemented!()
    }

    fn check_rmig_core_table(&self) -> RmigEmptyResult {
        unimplemented!()
    }

    fn create_rmig_core_table(&self) -> RmigEmptyResult {
        info!("Creating core table.");
        let table = if self.schema_admin.ne("") {
            let mut map = HashMap::<String, String>::new();
            map.insert("SCHEMA_ADMIN".to_string(), self.schema_admin.clone());
            let table = include_str!("../init/ora_init.sql");
            TeraManager::new(map).apply("core.sql", table)?
        } else {
            include_str!("../init/ora_init.sql").to_string()
        };
        let mut conn = self.pool.get().expect("Error while getting connection");

        conn.execute(&*table, &[])
            .map_err(|e| Error::SQLError(format!("{:?}", e)))?;

        if !conn.autocommit() {
            conn.commit()
                .map_err(|e| Error::SQLError(format!("{:?}", e)))?;
        }

        Ok(())
    }

    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    fn close(&self) {
        // FIXME: Do nothing.
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


#[cfg(test)]
mod local_test {
    use crate::configuration_properties::DatasourceProperties;
    use crate::driver::oracle::DatasourceOracle;
    use crate::driver::DriverFactory;
    use crate::test_utils::init_logger;

    fn create_local_connection() -> DatasourceOracle {
        init_logger();
        let url = std::env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres:example@localhost:5432/postgres".to_owned());
        let properties = DatasourceProperties::new(Some("Local pg ds".to_string()), url, None);
        DatasourceOracle::new(&properties)
    }
}