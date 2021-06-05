use crate::driver::RmigEmptyResult;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::Config;
use log4rs::config::{Appender, Root};
use log::LevelFilter;
use crate::error::Error;

pub fn init_logger() -> RmigEmptyResult {
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
    log4rs::init_config(config).map_err(|e| Error::LoggerConfigurationError(e.to_string()))?;
    Ok(())
}