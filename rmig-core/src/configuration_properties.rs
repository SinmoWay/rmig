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

impl DatasourceProperties {
    pub fn new(name: Option<String>, _url: String, properties: Option<HashMap<String, String>>) -> Self {
        DatasourceProperties {name, properties, full_url: Some(_url) }
    }
}
