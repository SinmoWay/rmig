use crate::changelogs::{Migration, Query};
use crate::configuration_properties::DatasourceProperties;
use crate::driver::{
    generate_lock, DatasourceWrapper, Driver, DriverFactory, DriverOptions, RmigEmptyResult,
};
use crate::error::Error;
use crate::tera_manager::TeraManager;
use async_trait::async_trait;
use futures::executor::block_on;
use futures::TryFutureExt;
use log::{debug, error, info};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgRow};
use sqlx::{PgPool, Row};
use std::borrow::Borrow;
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::time::Instant;

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
        let wrapper = DatasourceWrapper::new(Box::new(props.to_owned()));
        let url = wrapper.get_url();
        let name = wrapper.get_name();

        let pool_opts = PgPoolOptions::new();
        let conn_opts = PgConnectOptions::from_str(url)
            .map_err(|e| { Error::CreatingDatasourceError(format!("Url is not valid. Check your configuration and url parameters. Datasoruce name: {}\nError: {:?}", &name, e).to_string()) })
            .unwrap();
        let pool = Box::new(
            block_on(pool_opts.connect_with(conn_opts))
                .map_err(|e| {
                    Error::CreatingDatasourceError(
                        format!(
                            "Datasource is not configured or not working. {}\nError: {:?}",
                            &name, e
                        )
                        .to_string(),
                    )
                })
                .unwrap(),
        );

        let postgres = DatasourcePostgres {
            name,
            pool,
            schema_admin: wrapper.get_schema_admin(),
            separator: wrapper.get_separator(),
        };

        &postgres
            .validate_connection()
            .expect(format!("Failed creating datasource by name: {}", &*postgres.name).as_str());

        postgres
    }
}

#[async_trait]
impl Driver for DatasourcePostgres {
    fn validate_connection(&self) -> RmigEmptyResult {
        block_on(sqlx::query("SELECT 1").execute(self.pool.borrow()))
            .expect("Error while checking connection");
        info!("Connection pool success creating.\nPing query (select 1) is success.");
        Ok(())
    }

    // Sync function
    // TODO: Check pipeline query and analyze.
    fn migrate(&self, query: VecDeque<&Query>) -> RmigEmptyResult {
        let mut tx = block_on(self.pool.begin())
            .map_err(|e| Error::SQLError(format!("{:?}.\nOpen transaction failed.", e)))?;
        let start = Instant::now();

        for q in query {
            debug!("Running sql: {}", &*q.query);
            // Execute, if error, rollback transaction
            block_on(sqlx::query(&*q.query).execute(&mut tx))
                .map_err(|e| Error::SQLError(format!("{:?}.\nSQL - {}", e, &*q.query)))?;
        }

        block_on(tx.commit())
            .map_err(|e| Error::SQLError(format!("{:?}.\nCommit transaction failed.", e)))?;

        let elapsed = start.elapsed();

        debug!("Success! Time elapsed: {:?}", elapsed);

        Ok(())
    }

    fn find_in_core_table(&self, name: String, hash: String) -> RmigEmptyResult {
        let schema = &self.schema_admin;
        let sep = &self.separator;
        let sql = format!("select exists(select 1 from {}{}CHANGELOGS where FILENAME=$1) as erow, exists(select 1 from {}{}CHANGELOGS where FILENAME=$1 and HASH = $2) as erowhash", schema, sep, schema, sep);
        // language=SQL
        let query: PgRow = block_on(
            sqlx::query(&*sql)
                .bind(&name)
                .bind(&hash)
                .fetch_one(&*self.pool),
        )
        // language=RUST
        .map_err(|e| {
            Error::SQLError(
                format!(
                    "Row with filename {} and hash {} return error.\nError: {:?}",
                    &name, &hash, e
                )
                .to_string(),
            )
        })?;

        let erow: bool = query.get("erow");
        // If row exists find row with hash.
        if !erow {
            return Err(Error::RowError(
                format!("Row with filename {} not found.", &name).to_string(),
            ));
        }

        let erowhash: bool = query.get("erowhash");
        // If row with hash and name not found, but row with filename found, hash has been changed.
        if !erowhash {
            return Err(Error::HashUniqueError(format!("Row with filename {} and hash {} not found. Hash has been changed. Please revert your changed, or set migration options on run=always.", &name, &hash).to_string()));
        }

        Ok(())
    }

    fn check_rmig_core_table(&self) -> RmigEmptyResult {
        let sub_query = if self.schema_admin.ne("") {
            format!(" AND SCHEMANAME = '{}'", &*self.schema_admin)
        } else {
            "".to_string()
        };
        let ex = block_on(sqlx::query(format!("SELECT EXISTS(SELECT 1 FROM pg_tables WHERE tablename = 'CHANGELOGS' or tablename = 'changelogs'{}) as ex", sub_query).as_str())
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
        let table = if self.schema_admin.ne("") {
            let mut map = HashMap::<String, String>::new();
            map.insert("SCHEMA_ADMIN".to_string(), self.schema_admin.clone());
            let table = include_str!("../init/pg_init.sql");
            TeraManager::new(map).apply("core.sql", table)?
        } else {
            TeraManager::default().apply("core.sql", include_str!("../init/pg_init.sql"))?
        };

        block_on(sqlx::query(&*table).execute(&*self.pool))
            .map_err(|e| Error::SQLError(format!("{:?}", e)))?;
        Ok(())
    }

    fn get_name(&self) -> &str {
        self.name.as_str()
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
            .await
            .map_err(|e| Error::SQLError(format!("{:?}", e)))?;

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
            .await
            .map_err(|e| Error::SQLError(format!("{:?}", e)))?;

        Ok(())
    }

    async fn add_new_migration(&self, migration: Migration) -> RmigEmptyResult {
        let _sql = format!(
            "INSERT INTO {}{}CHANGELOGS(FILENAME, ORDER_ID, HASH) VALUES ($1,$2,$3);",
            &*self.schema_admin, &*self.separator
        );
        sqlx::query(&*_sql)
            .bind(&*migration.name)
            .bind(migration.order.clone())
            .bind(&*migration.hash)
            .execute(self.pool.borrow())
            .await
            .map_err(|e| Error::SQLError(format!("{:?}", e)))?;
        Ok(())
    }
}

async fn current_database(pool: &PgPool) -> anyhow::Result<String, Error> {
    // language=SQL
    Ok(sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        // language=RUST
        .await
        .map_err(|e| Error::SQLError(format!("{:?}", e)))?)
}

impl Drop for DatasourcePostgres {
    fn drop(&mut self) {
        info!("Unlocking session.");
        block_on(self.unlock().unwrap_or_else(|_e| {
            error!("Unlocking session return error code.");
        }));
        info!("Closing pool {}", &self.get_name());
        self.close()
    }
}

// postgres
#[cfg(all(test))]
mod local_test {
    use crate::changelogs::{Migration, Query};
    use crate::configuration_properties::DatasourceProperties;
    use crate::driver::postgres::DatasourcePostgres;
    use crate::driver::{Driver, DriverFactory, RmigEmptyResult};
    use crate::error::Error;
    use futures::executor::block_on;
    use log::LevelFilter;
    use log4rs::append::console::{ConsoleAppender, Target};
    use log4rs::config::{Appender, Root};
    use log4rs::Config;
    use sqlx::Row;
    use std::collections::VecDeque;

    #[test]
    pub fn test_crc_32() {
        let mut x = crc32fast::Hasher::new();
        x.update(b"asdfasdfasdfasdf");
        let hash = x.finalize();
        println!("x = {}", hash);
        assert_eq!(1076699909, hash as i64)
    }

    #[test]
    pub fn check_connection() -> RmigEmptyResult {
        let postgres = create_local_connection();
        assert_eq!((), postgres.validate_connection()?);
        assert_eq!((), block_on(postgres.lock())?);
        assert_eq!((), block_on(postgres.unlock())?);
        Ok(())
    }

    #[test]
    pub fn create_core_table() -> RmigEmptyResult {
        let postgres = create_local_connection();
        assert_eq!((), postgres.validate_connection()?);
        assert_eq!((), block_on(postgres.lock())?);

        postgres
            .check_rmig_core_table()
            .unwrap_or_else(|_e| postgres.create_rmig_core_table().unwrap());

        assert_eq!((), postgres.check_rmig_core_table()?);

        assert_eq!((), block_on(postgres.unlock())?);
        Ok(())
    }

    #[test]
    pub fn test_get_name() -> RmigEmptyResult {
        let postgres = create_local_connection();
        assert_eq!("localhost", postgres.get_name());
        Ok(())
    }

    #[test]
    pub fn migrate_and_add_new_migration() -> RmigEmptyResult {
        let postgres = create_local_connection();
        assert_eq!("localhost", postgres.get_name());
        assert_eq!((), postgres.validate_connection()?);

        let name = "test_dir".to_string();
        let hash = "md5".to_string();

        assert_eq!((), block_on(postgres.lock())?);
        let migration = create_migration(name.to_owned(), hash.to_owned());

        postgres
            .check_rmig_core_table()
            .unwrap_or_else(|_e| postgres.create_rmig_core_table().unwrap());

        postgres
            .find_in_core_table(name.to_owned(), hash.to_owned())
            .unwrap_or_else(|e| {
                postgres
                    .migrate(migration.query_list.iter().map(|i| i).collect())
                    .unwrap();
                block_on(postgres.add_new_migration(migration)).unwrap();
            });

        let row: (i32,) = block_on(
            sqlx::query_as("SELECT 150 as result FROM rmig_test WHERE test = '123456'")
                .fetch_one(&*postgres.pool),
        )
        .unwrap();
        assert_eq!(150, row.0);

        assert_eq!((), block_on(postgres.unlock())?);

        // Clear information
        block_on(sqlx::query("DROP TABLE rmig_test").execute(&*postgres.pool)).unwrap();
        block_on(
            sqlx::query("DELETE FROM changelogs WHERE FILENAME = $1 AND HASH = $2")
                .bind(name.to_owned())
                .bind(hash.to_owned())
                .execute(&*postgres.pool),
        )
        .unwrap();
        Ok(())
    }

    fn create_migration(name: String, hash: String) -> Migration {
        let mut querys = VecDeque::new();
        querys.push_back(Query {
            query: "SELECT 1".to_string(),
            opts: Default::default(),
        });
        querys.push_back(Query {
            query: "CREATE TABLE rmig_test(test TEXT)".to_string(),
            opts: Default::default(),
        });
        querys.push_back(Query {
            query: "INSERT INTO rmig_test(test) VALUES('123456')".to_string(),
            opts: Default::default(),
        });
        Migration {
            name,
            hash,
            separator: "".to_string(),
            order: 1,
            query_list: querys,
            options: None,
        }
    }

    fn create_local_connection() -> DatasourcePostgres {
        init_logger();
        let url = std::env::var("PG_DB_URL")
            .unwrap_or("postgres://postgres:example@localhost:5432/postgres".to_owned());
        let properties = DatasourceProperties::new(Some("Local pg ds".to_string()), url, None);
        DatasourcePostgres::new(&properties)
    }

    fn init_logger() -> RmigEmptyResult {
        let stdout = ConsoleAppender::builder().target(Target::Stdout).build();
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(LevelFilter::Info))
            .map_err(|_e| {
                Error::LoggerConfigurationError(String::from(
                    "Configuration is empty or include another error.",
                ))
            })?;

        // Use this to change log levels at runtime.
        // This means you can change the default log level to trace
        // if you are trying to debug an issue and need more logs on then turn it off
        // once you are done.
        log4rs::init_config(config).map_err(|e| Error::LoggerConfigurationError(e.to_string()));
        Ok(())
    }
}
