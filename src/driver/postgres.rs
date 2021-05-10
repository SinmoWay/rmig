use sqlx::{Row, PgPool};
use log::{info, debug, error};
use crate::configuration_properties::DatasourceProperties;
use std::borrow::Borrow;
use crate::driver::{Driver, DriverFactory, DriverOptions, RmigEmptyResult, generate_lock};
use std::collections::{HashMap, VecDeque};
use sqlx::postgres::{PgPoolOptions, PgConnectOptions, PgRow};
use std::str::FromStr;
use futures::executor::block_on;
use crate::changelogs::{Query, Migration};
use std::time::{Instant};
use async_trait::async_trait;
use crate::tera_manager::TeraManager;
use futures::TryFutureExt;
use crate::error::Error;

#[derive(Clone, Debug)]
pub struct DatasourcePostgres {
    pub name: String,
    pub pool: Box<PgPool>,
    pub schema_admin: String,
    pub separator: String,
}

/// Creating datasource
/// TODO: Maybe impl async!
impl DriverFactory<DatasourcePostgres> for DatasourcePostgres {
    fn new(props: &DatasourceProperties) -> DatasourcePostgres {
        let url = props.full_url.as_ref().expect("Url for datasource is required.").as_str();
        let name = url::Url::parse(&*url).map_err(|_e| Error::CreatingDatasourceError("Url is not valid. Check your configuration and url parameters.".to_string())).unwrap().host_str().expect("Not found hostname.").to_owned();
        debug!("Creating datasource pool: {}", &*name);
        let pool_opts = PgPoolOptions::new();
        let conn_opts = PgConnectOptions::from_str(url).map_err(|e| { Error::CreatingDatasourceError(format!("Url is not valid. Check your configuration and url parameters. Datasoruce name: {}\nError: {:?}", &name, e).to_string()) }).unwrap();
        let pool = Box::new(block_on(pool_opts.connect_with(conn_opts)).map_err(|e| Error::CreatingDatasourceError(format!("Datasource is not configured or not working. {}\nError: {:?}", &name, e).to_string())).unwrap());
        let schema_admin = props.properties.as_ref().unwrap_or(HashMap::<String, String>::new().borrow()).get("SCHEMA_ADMIN").unwrap_or(&"".to_string()).to_string();
        let mut separator = "";
        if !schema_admin.is_empty() {
            separator = ".";
        }
        let postgres = DatasourcePostgres { name: name.clone(), pool, schema_admin, separator: separator.to_string() };
        &postgres.validate_connection().expect(format!("Failed creating datasource by name: {}", &*name).as_str());
        postgres
    }
}

#[async_trait]
impl Driver for DatasourcePostgres {
    fn validate_connection(&self) -> RmigEmptyResult {
        block_on(sqlx::query("SELECT 1").execute(self.pool.borrow())).expect("Error while checking connection");
        info!("Connection pool success creating.\nPing query (select 1) is success.");
        Ok(())
    }

    // Sync function
    // TODO: Check pipeline query and analyze.
    fn migrate(&self, query: VecDeque<&Query>) -> RmigEmptyResult {
        let mut tx = block_on(self.pool.begin()).map_err(|e| Error::SQLError(format!("{:?}.\nOpen transaction failed.", e)))?;
        let start = Instant::now();

        for q in query {
            debug!("Running sql: {}", &*q.query);
            // Execute, if error, rollback transaction
            block_on(sqlx::query(&*q.query).execute(&mut tx)).map_err(|e| Error::SQLError(format!("{:?}.\nSQL - {}", e, &*q.query)))?;
        };

        block_on(tx.commit()).map_err(|e| Error::SQLError(format!("{:?}.\nCommit transaction failed.", e)))?;

        let elapsed = start.elapsed();

        debug!("Success! Time elapsed: {:?}", elapsed);

        Ok(())
    }

    fn find_in_core_table(&self, name: String, hash: String) -> RmigEmptyResult {
        let schema = &self.schema_admin;
        let sep = &self.separator;
        let sql = format!("select exists(select 1 from {}{}CHANGELOGS where FILENAME=$1) as erow, exists(select 1 from {}{}CHANGELOGS where FILENAME=$1 and HASH = $2) as erowhash", schema, sep, schema, sep);
        // language=SQL
        let query: PgRow = block_on(sqlx::query(&*sql)
            .bind(&name)
            .bind(&hash)
            .fetch_one(&*self.pool))
            // language=RUST
            .map_err(|e| Error::SQLError(format!("Row with filename {} and hash {} return error.\nError: {:?}", &name, &hash, e).to_string()))?;

        let erow: bool = query.get("erow");
        // If row exists find row with hash.
        if !erow {
            return Err(Error::RowError(format!("Row with filename {} not found.", &name).to_string()));
        }

        let erowhash: bool = query.get("erowhash");
        // If row with hash and name not found, but row with filename found, hash has been changed.
        if !erowhash {
            return Err(Error::HashUniqueError(format!("Row with filename {} and hash {} not found. Hash has been changed. Please revert your changed, or set migration options on run=always.", &name, &hash).to_string()));
        }

        Ok(())
    }

    fn check_rmig_core_table(&self) -> RmigEmptyResult {
        let sub_query = if self.schema_admin.ne("") { format!(" AND SCHEMANAME = '{}'", &*self.schema_admin) } else { "".to_string() };
        let ex = block_on(sqlx::query(format!("SELECT EXISTS(SELECT 1 FROM pg_tables WHERE tablename = 'CHANGELOGS'{}) as ex", sub_query).as_str())
            // language=RUST
            .fetch_one(&*self.pool)).map_err(|e| Error::SQLError(format!("{:?}", e)))?;
        let x: bool = ex.get("ex");
        if x {
            Ok(())
        } else {
            Err(Error::RowError("Not found core table.".to_string()))
        }
    }

    fn create_rmig_core_table(&self) -> RmigEmptyResult {
        info!("Creating core table.");
        let table =
            if self.schema_admin.ne("") {
                let mut map = HashMap::<String, String>::new();
                map.insert("SCHEMA_ADMIN".to_string(), self.schema_admin.clone());
                let table = include_str!("../init/pg_init.sql");
                TeraManager::new(map).apply("core.sql", table)?
            } else { include_str!("../init/pg_init.sql").to_string() };

        block_on(sqlx::query(&*table).execute(&*self.pool)).map_err(|e| Error::SQLError(format!("{:?}", e)))?;
        Ok(())
    }

    fn get_name(&self) -> &str {
        self.borrow().name.as_str()
    }

    fn close(&self) {
        block_on(self.pool.close());
        if self.pool.is_closed() {
            debug!("Pool success closed.");
        }
    }

    async fn lock(&self) -> RmigEmptyResult {
        let database_name = current_database(self.pool.borrow()).await?;
        let lock_id = generate_lock(database_name);

        // Locking
        // https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS
        // https://www.postgresql.org/docs/current/functions-admin.html#FUNCTIONS-ADVISORY-LOCKS-TABLE

        // language=SQL
        let _ = sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(lock_id)
            .execute(self.pool.borrow())
            // language=RUST
            .await.map_err(|e| Error::SQLError(format!("{:?}", e)))?;

        Ok(())
    }

    async fn unlock(&self) -> RmigEmptyResult {
        let database_name = current_database(self.pool.borrow()).await?;
        let lock_id = generate_lock(database_name);

        // Unlocking
        // https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS
        // https://www.postgresql.org/docs/current/functions-admin.html#FUNCTIONS-ADVISORY-LOCKS-TABLE

        // language=SQL
        let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(lock_id)
            .execute(self.pool.borrow())
            // language=RUST
            .await.map_err(|e| Error::SQLError(format!("{:?}", e)))?;

        Ok(())
    }

    async fn add_new_migration(&self, migration: Migration) -> RmigEmptyResult {
        let _sql = format!("INSERT INTO {}{}CHANGELOGS(FILENAME, ORDER_ID, HASH) VALUES ($1,$2,$3);", &*self.schema_admin, &*self.separator);
        sqlx::query(&*_sql)
            .bind(&*migration.name)
            .bind(migration.order.clone())
            .bind(&*migration.hash)
            .execute(self.pool.borrow()).await.map_err(|e| Error::SQLError(format!("{:?}", e)))?;
        Ok(())
    }
}

async fn current_database(pool: &PgPool) -> anyhow::Result<String, Error> {
    // language=SQL
    Ok(sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        // language=RUST
        .await.map_err(|e| Error::SQLError(format!("{:?}", e)))?)
}

impl Drop for DatasourcePostgres {
    fn drop(&mut self) {
        info!("Unlocking session.");
        self.unlock().unwrap_or_else(|_e| {
            error!("Unlocking session return error code.");
        });
        info!("Closing pool {}", &self.get_name());
        self.close()
    }
}

#[cfg(test)]
mod local_test {
    #[test]
    pub fn test_crc_32() {
        let mut x = crc32fast::Hasher::new();
        x.update(b"asdfasdfasdfasdf");
        let hash = x.finalize();
        println!("x = {}", hash);
        assert_eq!(1076699909, hash as i64)
    }
}