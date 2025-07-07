use serde_yaml::Mapping;
use std::fs::File;
use std::io::prelude::*;

use crate::swagger::Swagger;

pub fn save_yaml(path: String, swagger: Swagger) {
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

    let yaml = serde_yaml::to_string(&map).unwrap();
    let mut file = File::create(path).expect("Error creating to file");
    write!(file, "{}", yaml).expect("Error writing to file");
}
