![Logo](assets/logo.svg)

Lightweight cross-platform data migration application.

[![MIT licensed][mit-badge]][mit-url]
![CI main branch][ci-main-badge]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg

[mit-url]: LICENSE

[ci-main-badge]: https://github.com/SinmoWay/rmig/actions/workflows/rmig-build-and-test.yml/badge.svg?branch=main

# Warning! The project is under development and code refactoring. Production use is not ready.

## Database Support

* Postgres
* MySQL
* Oracle

## Usage

TODO

## Migration version

TODO

## Future and roadmap

[Read on Wiki page](https://github.com/SinmoWay/rmig/wiki/Roadmap#release-100)

## Support

TODO

## Changelog specification.

### Configuration file's

For the description of the changelog, we use yml files. In the future, we will also support json. In case you want additional support from your IDE, import rmig-changelog-spec.json from the assets directory.

### Changelog table

Changelog tables are described in the core module. There is also support for creating a table not in the root element, for this it is enough to declare env or properties with the name SCHEMA_ADMIN. For an introduction, see the postgres table creation file: `rmig-core/src/init/pg_init.sql`.

### Сhangelog locking mechanism 

The locking mechanism depends entirely on the type of driver. We are trying to do without additional tables, and depending on the DBMS, we are trying to create a unique lock. For example: for we will use postgres, we will execute code like this:
```
--------------------------------
        // language=SQL
        let _ = sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(lock_id)
            .execute(self.pool.borrow())
            // language=RUST
            .await.map_err(|e| Error::SQLError(format!("{:?}", e)))?;
--------------------------------            
```
For oracle, we will use DBMS_LOCK package, for MySQL - GET_LOCK.


The lock is generated based on the migration object (schema name or database name), for example:
```
--------------------------------
fn generate_lock(db_name: String) -> i64 {
    let mut x = crc32fast::Hasher::new();
    x.update(db_name.as_bytes());
    x.finalize() as i64
}
--------------------------------
```

## Build and testing

### Build

To build the project, we use cargo and the toolchain version - nightly.

Firstly, we need install nightly toolchain:
``
rustup toolchain install nightly
``

Second action, build project:
``
cargo build --all-features
``

### Tests

For run test in all modules, use command:
``
cargo test --all
``

Core module has bench tests. To run them, you need a ``` cargo bench ```, you can view the generated report using the criterion.

The application has integration tests, in case you want to debug some general interaction, use docker-compose in the .docker folder. If you do not have oracle locally installed, you can use files in the .docker/oracle bat/sh folder to build the image.
Location: `test-integration/`

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in refinery by you, shall
be licensed as MIT, without any additional terms or conditions.