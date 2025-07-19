use indexmap::IndexMap;
use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Swagger {
    pub swagger: String,
    pub info: Info,
    pub base_path: String,
    pub tags: Vec<Tag>,
    pub paths: Paths,
    pub definitions: Definitions,
}

pub type Paths = IndexMap<String, HashMap<String, Value>>;
pub type Definitions = IndexMap<String, Definition>;
pub type Definition = IndexMap<String, Value>;

#[derive(Deserialize, Serialize)]
pub struct Info {
    pub title: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Tag {
    pub name: String,
    pub description: String,
}

pub fn get_swagger(url: String, headers: HashMap<String, String>) -> Swagger {
    let mut header_map = HeaderMap::new();
    for (key, val) in headers {
        match (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_str(val.as_str()),
        ) {
            (Ok(name), Ok(value)) => {
                header_map.insert(name, value);
            }
            _ => println!("Ошибка при преобразовании {}={}", key, val),
        }
    }

    let client = reqwest::blocking::Client::new();
    let body = client
        .get(url)
        .headers(header_map)
        .send()
        .expect("request failed")
        .text()
        .expect("body failed");

    serde_json::from_str(&body).unwrap()
}
