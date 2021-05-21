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

## The problems we are trying to solve
To begin with, I would like to describe the problems that I would like to solve.
First, standard libraries significantly complicate a relatively simple process such as data migration. If you live in a microservice structure, you should understand what it is like to have a database in each service. And in large services, we can face thousands of migrations and enter each of them in the changelog ... This is really a pain. And here is my first goal: to create the most laconic changelog. At the moment I got to this:
```
changelogs:
  - name: Initialize
    directory: ./migration/init/**
  - name: Drop
    directory: ./drop/**
```
We support both direct links to files or directories and wildcard. We are not asking you to describe any additional parameters, we will only need ... The stage - `name` and the path to the directory.
Secondly, quite often there is a situation that after the dev environment, it is necessary to roll out migrations at several additional stands. And what should be done? We manually or someone automatically runs it through scripts. All this takes time. And here's the second goal: to be able to run migrations on several databases, while doing it in parallel (currently in the process of implementation).
By convention, the most advanced changelog should look something like this:
```
changelogs:
  - name: Initialize
    directory: ./migration/init/**
  - name: Drop
    directory: ./migration/drop/**

properties:
  key: value
  hello: wolrd
  admin_schema: "{{ schema_name }}"

datasources:
  - name: test1
    url: test
    properties:
      blob: no
  - name: test2
    url: test
    ...
```
At the same time, no one forbids you to template both changelog and migration. We use tera template engine for such purposes.

## Usage

### General
First, you need to create a changelog, it must follow the `rmig-changelog-spec.json` schema. You can also reorganize your migrations or changelog (if the project is new), and use the engine template [Tera](https://github.com/Keats/tera).
Next, you can describe the DB or your configuration in your changelog or use the command line for this.

### Naming rules for migrations
There is only one rule, use the naming order for migrations. The general pattern is: `ORDER.your file name with extension`. Thus, the order specifies the order of performing migrations in the directory.

#### Special cases
Consider this situation:
```
--- migration <- FOLDER
    | --- 1.user_folder <- FOLDER
        | --- 1.....sql
        | --- n.....sql
    | --- 2.any_folder <- FOLDER
        | --- 1.....sql
        | --- n.....sql
    | --- 1.init.sql
    | --- 2.create_dummy.sql
    | --- 3.create_hello.sql        
--- changelog.yml
```
... and changelog
```
changelogs:
  - name: Initialize
    directory: ./migration/**
datasources:
  - name: my_best_datasource
    url: {{ DB }}    
```
After that, let's call `rmig run -с changelog.yml -s Initialize -e DB=My_Datasource_Full_Url`. 

The question arises: what is the sequence of the call? If we describe the sequence of the call, then it will be something like this:
* We start reading recursively each folder and add any file to the end of the list.
* If we come across a directory, then first we read the files in the directory itself, and only after that we read the files in the sub directories.  
Accordingly, first we will read the files `1.init.sql`, `2.create_dummy.sql`, `3.create_hello.sql`, and add them to the list. Next, we will face the first directory `1.user_folder`
and start reading it recursively (by analogy with the beginning), and after `2.any_folder`.
  
We must understand that when reading files/directories, we also resolve all templates inside. Also, the `ORDER` value cannot be less than zero.


### Cli commands
General configuration flags and parameters:
```
FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <config>          Config file, supports yaml and json, see changelog.yml for more details. This env, override, and have high priority on\":\" --url, --env properties.
    -d, --debug <logging_level>    Set logger level on Warn/Trace/Debug/Info/Error. Default level Info.
    -e, --env <properties>...      Properties for configuration and replacement (placeholders {{ you_placeholder }})
```

#### Run migration
To start migrations, you need to write the command `rmig run -с changelog1 -c changelog2 ...` for more information use --help.
```
OPTIONS:
    -s, --stages <stage>...    Stage name. Execution by order. Maybe multiply.
        --url <url>            Url for database
```

#### Status migration
TODO

## Future and roadmap

[Read on Wiki page](https://github.com/SinmoWay/rmig/wiki/Roadmap#release-100)

## Changelog specification.

### Configuration file's

For the description of the changelog, we use yml files. In the future, we will also support json. In case you want additional support from your IDE, import `rmig-changelog-spec.json` from the `assets` directory.

### Changelog table

Changelog tables are described in the core module. There is also support for creating a table not in the root element, for this it is enough to declare env or properties with the name `SCHEMA_ADMIN`. For an introduction, see the postgres table creation file: `rmig-core/src/init/pg_init.sql`.

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
For oracle, we will use `DBMS_LOCK` package, for MySQL - `GET_LOCK`.


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