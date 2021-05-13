use clap::{load_yaml, App, ArgMatches};
use crate::configuration_properties::{DatasourceProperties, DatasourcesProperties};
use std::borrow::Borrow;
use log::LevelFilter;
use std::collections::{HashMap};
use log4rs::config::{Root, Appender};
use log4rs::{Config, Handle};
use log4rs::append::console::{ConsoleAppender, Target};
use crate::driver::{Driver, DatasourceFactory, RmigEmptyResult};
use crate::tera_manager::TeraManager;
use crate::changelogs::{ChangelogRunner, Changelog, Directory};
use futures::executor::block_on;
use log::{info, error};
use crate::error::Error;
use crate::enum_str;
use std::str::FromStr;

enum_str! {
 pub enum Command {
    Status = 0x00000,
    Run = 0x00001,
 }
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let command = s.to_lowercase();
        return if Command::Status.name().to_lowercase().eq(command.as_str()) {
            Ok(Command::Status)
        } else {
            Ok(Command::Run)
        };
        // Err(Error::NotFoundCommand(s.to_string()))
    }
}

#[derive(Clone, Debug)]
pub struct Cli {
    args: Option<ArgMatches>,
}

impl Cli {
    pub fn new() -> Self {
        Cli { args: None }
    }
}

impl Default for Cli {
    fn default() -> Self {
        Cli::new()
    }
}

impl Cli {
    pub fn get_matches(mut self) -> ArgMatches {
        return if self.args.is_none() {
            let yaml = load_yaml!("cli.yml");
            let matches = App::from(yaml).get_matches();
            self.args.insert(matches.clone());
            return matches;
        } else {
            self.args.unwrap()
        };
    }
}

#[derive(Clone)]
pub struct CliArgs {
    command: Option<Command>,
    logging_level: Option<LevelFilter>,
    url: Option<String>,
    config: Option<String>,
    stage: Option<Vec<String>>,
    properties: Option<HashMap<String, String>>,
}

impl Default for CliArgs {
    fn default() -> Self {
        CliArgs {
            command: None,
            logging_level: None,
            url: None,
            config: None,
            stage: None,
            properties: None,
        }
    }
}

/**
 Processing configuration by setting's on [cli.yml]. Reading and converting by objects.
*/
#[derive(Clone)]
pub struct CliReader {
    args: CliArgs,
    args_match: ArgMatches,
}

/// CliProcessor is implemented builder pattern and getting functionality from checking parameters
impl CliReader {
    /// Read subcommands [Command enum]
    pub fn read_command(mut self) -> CliReader {
        let command = match self.args_match.subcommand_name() {
            None => { panic!("{}", "Not found command to execute. rmig --help for more information.") }
            Some(s) => { Command::from_str(s) }
        }.unwrap();
        self.args.command.insert(command);
        self
    }

    /// Read properties [--logging_level/-d]
    pub fn read_logging_level(mut self) -> CliReader {
        let level_filter = match self.args_match.value_of("logging_level") {
            None => { LevelFilter::Info }
            Some(level) => {
                LevelFilter::from_str(&*level).unwrap_or(LevelFilter::Info)
            }
        };
        self.args.logging_level.insert(level_filter);
        self
    }

    /// Read properties [--config/-c]
    pub fn read_config(mut self) -> CliReader {
        self.args_match.value_of("config").map(|arg| String::from(arg)).map(|arg| self.args.config.insert(arg));
        self
    }

    /// Read properties [--url]
    pub fn read_url(mut self) -> CliReader {
        if let Some(c) = self.args.command.as_ref() {
            if Command::Run == c.clone() {
                if let Some(m) = self.args_match.subcommand_matches(Command::Run.name().to_lowercase()) {
                    m.value_of("url").map(|arg| String::from(arg)).map(|arg| self.args.url.insert(arg));
                }
            }
        }
        self
    }

    /// Read properties [--stage/-s]
    pub fn read_stage(mut self) -> CliReader {
        if let Some(c) = self.args.command.as_ref() {
            if Command::Run == c.clone() {
                if let Some(m) = self.args_match.subcommand_matches(Command::Run.name().to_lowercase()) {
                    let stage = m.values_of("stage")
                        .map(|value| value.into_iter().map(|v| String::from(v)).collect::<Vec<String>>());
                    self.args.stage = stage;
                }
            }
        }
        self
    }

    /// Read properties [--env/-e]
    pub fn read_properties(mut self) -> CliReader {
        let mut _properties = HashMap::<String, String>::new();
        self.args_match.values_of("properties").map(|value| {
            value.into_iter().for_each(|arg| {
                let kv = arg.split("=").collect::<Vec<&str>>();
                _properties.insert(kv[0].to_owned(), kv[1].to_owned());
            });
        });

        if !_properties.is_empty() {
            self.args.properties.insert(_properties);
        }

        self
    }

    /// Read all configuration properties and return this instance
    pub fn read(self) -> CliReader {
        self.read_command()
            .read_logging_level()
            .read_config()
            .read_stage()
            .read_url()
            .read_properties()
    }

    pub fn args(&self) -> &CliArgs {
        &self.args
    }
}

impl Default for CliReader {
    fn default() -> Self {
        CliReader {
            args: CliArgs::default(),
            args_match: Cli::default().get_matches(),
        }.read()
    }
}

pub struct AppRmigCli {
    args: CliArgs,
    arg_processor: CliReader,
    logging_handler: Option<Handle>,
    datasources: Vec<Box<dyn Driver>>,
}

impl AppRmigCli {
    pub fn init(self) -> AppRmigCli {
        self.read_args().logging_level().read_datasource()
    }

    fn read_args(mut self) -> AppRmigCli {
        self.args = self.arg_processor.args().to_owned();
        self
    }

    pub async fn execute(&mut self) -> anyhow::Result<(), Error> {
        let command = self.args.command.as_ref().unwrap();
        return if Command::Run == *command {
            self.run().await
        } else if Command::Status == *command {
            self.status().await
        } else {
            Err(Error::NotFoundCommand("Command not found.".to_string()))
        };
    }

    pub async fn status(&mut self) -> anyhow::Result<(), Error> {
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<(), Error> {
        let ds_v = self.datasources.iter().map(|i| i).collect::<Vec<&Box<dyn Driver>>>();
        let props = *&self.args.properties.as_ref().unwrap();

        let mut future_drivers = Vec::with_capacity(ds_v.len());

        // First action, prepare table if does not exists
        for driver in &self.datasources {
            let d = prepare_db(driver);
            future_drivers.push(d);
        }

        let config = &self.args.config.as_ref();

        // If stages is empty, skip filtering.
        let stages = self.args.stage.clone().unwrap_or(vec![]);

        // Awaiting
        info!("Awaiting all datasources.");
        for driver in future_drivers {
            driver.await?;
        }

        let impl_changelogs_from_cfg = if config.is_some() {
            Ok(ChangelogRunner::new_from_file(config.unwrap().clone(), ds_v.clone(), Some(props.clone())))
        } else {
            Err(Error::ParseFileError("File is empty or not readable.".to_string()))
        }?.filter_by_stage(stages.clone());

        let changelogs = impl_changelogs_from_cfg.changelog.changelogs;

        for changelog in changelogs {
            run_changelog(&ds_v, changelog).await
        }

        async fn prepare_db(driver: &Box<dyn Driver>) -> RmigEmptyResult {
            driver.check_rmig_core_table().or_else(|_| {
                driver.create_rmig_core_table()
            })
        }

        async fn run_changelog(driver: &Vec<&Box<dyn Driver>>, changelog: Changelog) {
            driver.iter().for_each(
                |d| {
                    let md = *d;
                    // Waiting lock.
                    // If lock is already exists, we await.
                    // Or else, lock session, and go migration.
                    block_on(md.lock()).unwrap();
                    read_dir(md, Box::new(changelog._directory.borrow())).unwrap();

                    fn read_dir(md: &Box<dyn Driver>, _directory: Box<&Directory>) -> anyhow::Result<(), Error> {
                        // Run current migration list.
                        _directory.migration_list.iter().for_each(
                            |m| {
                                let hash = &m.hash;
                                let name = &m.name;

                                match md.find_in_core_table(name.to_string(), hash.to_string()) {
                                    Ok(_) => {
                                        info!("Migration with {} with hash {} is already exists.", &*name, &*hash);
                                    }
                                    Err(e) => match e {
                                        // Ignoring other error's.
                                        Error::NotFoundCommand(_) => {}
                                        Error::CreatingDatasourceError(_) => {}
                                        Error::LoggerConfigurationError(_) => {}
                                        Error::ParseError(_, _) => {}
                                        Error::IOError(_) => {}
                                        Error::ParseFileError(_) => {}
                                        Error::ConnectionValidationError(_) => {}
                                        /////////////////////////////////////
                                        Error::SQLError(s) => {
                                            error!("Connection not stable, or query error.");
                                            panic!("{}", s);
                                        }
                                        Error::HashUniqueError(s) => {
                                            // panic, hash has been changed.
                                            error!("Hash has been changed.");
                                            panic!("{}", s);
                                        }
                                        Error::RowError(_s) => {
                                            // Row not found. Add new migration.
                                            info!("Run new migration with name: {}", &*m.name);
                                            // Panic and rollback transaction.
                                            md.migrate(m.query_list.iter().map(|i| i).collect()).unwrap();
                                            block_on(md.add_new_migration(m.clone())).unwrap();
                                        }
                                    },
                                }
                            }
                        );

                        if _directory._directory.is_some() {
                            let dirs = _directory._directory.as_ref().unwrap();
                            dirs.as_ref().iter().for_each(|d| read_dir(md, Box::new(d)).unwrap());
                        }
                        Ok(())
                    }
                }
            );
        }

        Ok(())
    }

    fn logging_level(mut self) -> AppRmigCli {
        fn _logging_level(level_filter: LevelFilter) -> anyhow::Result<Handle, Error> {
            let stdout = ConsoleAppender::builder().target(Target::Stdout).build();
            let config = Config::builder()
                .appender(Appender::builder().build("stdout", Box::new(stdout)))
                .build(
                    Root::builder()
                        .appender("stdout")
                        .build(level_filter.to_owned()),
                )
                .map_err(|_e| Error::LoggerConfigurationError(String::from("Configuration is empty or include another error.")))?;

            // Use this to change log levels at runtime.
            // This means you can change the default log level to trace
            // if you are trying to debug an issue and need more logs on then turn it off
            // once you are done.
            log4rs::init_config(config).map_err(|e| Error::LoggerConfigurationError(e.to_string()))
        }
        let level_filter = self.args.logging_level.unwrap_or(LevelFilter::Info);
        self.logging_handler.insert(_logging_level(level_filter).expect("Logging level is not known."));
        self
    }

    fn read_datasource(mut self) -> AppRmigCli {
        let datasources = match self.args.config.as_ref() {
            None => {
                read_datasource_properties(self.args.url.as_ref().unwrap().clone(), self.args.properties.clone())
            }
            Some(_config) => {
                read_datasource_properties_from_file(_config.clone(), self.args.properties.clone()).unwrap()
            }
        };

        fn read_datasource_properties(url: String, properties: Option<HashMap<String, String>>) -> Vec<DatasourceProperties> {
            let dsp = DatasourceProperties::new(None, url, properties);
            vec![dsp]
        }

        fn read_datasource_properties_from_file(path: String, properties: Option<HashMap<String, String>>) -> anyhow::Result<Vec<DatasourceProperties>, Error> {
            let mut yaml = std::fs::read_to_string(path.as_str()).map_err(|e| Error::IOError(e.to_string()))?;

            if properties.as_ref().is_some() {
                yaml = TeraManager::new(properties.as_ref().unwrap().clone()).apply("changelogs.yml", yaml.as_str())?;
            }

            let datasources: DatasourcesProperties = serde_yaml::from_str(yaml.as_str())
                .map_err(|e| Error::ParseError(path.to_owned(), e.to_string()))?;
            Ok(datasources.datasources)
        }

        // All datasources unwrap and raise error (panic)
        fn create_datasource(mut properties: Vec<DatasourceProperties>, props: Option<HashMap<String, String>>) -> Vec<Box<dyn Driver>> {
            properties.iter_mut().map(|p| {
                if p.properties.as_ref().is_some() {
                    let mut option = p.properties.clone().unwrap();
                    if props.is_some() {
                        option.extend(props.clone().unwrap());
                        p.properties.insert(option.clone());
                    }
                } else {
                    if props.is_some() {
                        p.properties.insert(props.clone().unwrap());
                    }
                }
                DatasourceFactory::new(p).unwrap()
            }).collect::<Vec<Box<dyn Driver>>>()
        }

        self.datasources = create_datasource(datasources, self.args.properties.clone());
        self
    }
}

impl Default for AppRmigCli {
    fn default() -> Self {
        AppRmigCli { args: CliArgs::default(), arg_processor: CliReader::default(), logging_handler: None, datasources: Vec::new() }
    }
}
