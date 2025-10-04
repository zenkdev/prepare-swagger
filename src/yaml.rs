use indexmap::IndexMap;
use serde::Deserialize;
use serde_yaml::{Mapping, Value};
use std::collections::HashMap;

use crate::swagger::Swagger;

#[derive(Deserialize)]
pub struct Config {
  pub url: String,
  pub request: Option<RequestConfig>,
  pub paths: Option<PathsConfig>,
  pub definitions: Option<IndexMap<String, Value>>,
}

#[derive(Deserialize, Clone)]
pub struct RequestConfig {
  pub headers: Option<HashMap<String, String>>,
}

pub type PathsConfig = IndexMap<String, Value>;

pub fn get_yaml(swagger: Swagger) -> String {
  let mut map = Mapping::new();
  map.insert("swagger".into(), swagger.swagger.into());

  let mut info = Mapping::new();
  info.insert("version".into(), "1.0.0".into());
  info.insert("title".into(), swagger.info.title.into());
  map.insert("info".into(), info.into());

  map.insert("host".into(), "lgs.hostname".into());
  map.insert("basePath".into(), swagger.base_path.into());

  let mut tags: Vec<Mapping> = Vec::new();
  for tag in swagger.tags {
    let mut yaml_tag = Mapping::new();
    yaml_tag.insert("name".into(), tag.name.into());
    yaml_tag.insert("description".into(), tag.description.into());
    tags.push(yaml_tag);
  }
  map.insert("tags".into(), tags.into());

  map.insert("paths".into(), serde_yaml::to_value(swagger.paths).unwrap());
  map.insert(
    "definitions".into(),
    serde_yaml::to_value(swagger.definitions).unwrap(),
  );

  serde_yaml::to_string(&map).unwrap()
}
