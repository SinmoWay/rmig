#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    #[error("Illegal command - {0}. Use rmig --help for more information.")]
    NotFoundCommand(String),
    #[error("Creating DB pool error for url '{0}'.\nPlease check you parameters and try again.")]
    CreatingDatasourceError(String),
    #[error("Logging configuration return exit code. Please try again and set -d.\nError: {0}")]
    LoggerConfigurationError(String),
    #[error("Resolving throw error. Check template '{0}' and resolve exception. Cause: {1}")]
    ParseError(String, String),
    #[error("IOError. Read/Write file or dir is not support. {0}")]
    IOError(String),
    #[error("Reading file error.{0}")]
    ParseFileError(String),
    #[error("Execution query exception. {0}")]
    RowError(String),
    #[error("Validation connection return error. {0}")]
    ConnectionValidationError(String),
    #[error("Sql error. '{0}'")]
    SQLError(String),
    #[error("File with name {0} exists, but hash have been changed.")]
    HashUniqueError(String),
}