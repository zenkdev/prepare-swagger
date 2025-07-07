use indexmap::IndexMap;
use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderValue};
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
    pub definitions: IndexMap<String, Definition>,
}

type Definition = IndexMap<String, Value>;

#[derive(Deserialize, Serialize)]
pub struct Info {
    pub title: String,
}

#[derive(Deserialize, Serialize)]
pub struct Tag {
    pub name: String,
    pub description: String,
}

pub type Paths = IndexMap<String, HashMap<String, Value>>;

pub fn get_swagger(url: String, multienv: String) -> Swagger {
    let mut headers = HeaderMap::new();
    headers.insert("x-sc-lb-hint", HeaderValue::from_str(&multienv).unwrap());

    let client = reqwest::blocking::Client::new();
    let body = client
        .get(url)
        .headers(headers)
        .send()
        .expect("request failed")
        .text()
        .expect("body failed");

    serde_json::from_str(&body).unwrap()
}
