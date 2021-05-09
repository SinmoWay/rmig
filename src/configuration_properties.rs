use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasourcesProperties {
    pub datasources: Vec<DatasourceProperties>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasourceProperties {
    pub name: Option<String>,
    #[serde(rename = "url")]
    pub full_url: Option<String>,
    /// Unrecognized parameters for pg/mysql/sqlite
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CmdArg {
    pub name: String,
    pub about: String,
    pub short: Option<String>,
    pub long: String,
    pub takes: Option<bool>,
    pub multiply: Option<bool>,
    pub default: Option<String>,
    pub conflict_on: Option<String>,
}

impl DatasourceProperties {
    pub fn new(name: Option<String>, _url: String, properties: Option<HashMap<String, String>>) -> Self {
        DatasourceProperties {name, properties, full_url: Some(_url) }
    }
}
