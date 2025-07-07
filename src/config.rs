use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

#[derive(Deserialize)]
pub struct Config {
    pub schemas: Vec<Schema>,
}

#[derive(Deserialize, Clone)]
pub struct Schema {
    pub file: String,
    pub url: String,
    pub multienv: Option<String>,
    pub paths: Option<SchemaPaths>,
    pub definitions: Option<SchemaDefinitions>,
}

#[derive(Deserialize, Clone)]
pub struct SchemaPaths {
    pub __rename: Option<HashMap<String, String>>,
    pub __remove: Option<Vec<String>>,
}

#[derive(Deserialize, Clone)]
pub struct SchemaDefinitions {
    pub __add: Option<HashMap<String, Definition>>,
    pub __override: Option<HashMap<String, Definition>>,
    pub __remove: Option<Vec<String>>,
}

type Definition = IndexMap<String, Value>;

pub fn get_config(path: String) -> Config {
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let config: Config = serde_json::from_str(&contents).unwrap();

    config
}
