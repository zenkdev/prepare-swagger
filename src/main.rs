mod merge_map;
mod swagger;
mod yaml;

use convert_case::{Case, Casing};
use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::merge_map::merge;
use crate::swagger::get_swagger;
use crate::swagger::{Definitions, Paths};
use crate::yaml::read_config;
use crate::yaml::write_swagger;
use crate::yaml::{Config, RequestConfig};

fn main() {
    let usage = format!(
        "Usage: {} [path_to_config_file]",
        std::env::args().next().unwrap()
    );
    let path = std::env::args().nth(1).expect(&usage);

    let config = read_config(path);
    process_config(config);
}

fn process_config(config: Config) {
    let request = config
        .request
        .unwrap_or_else(|| RequestConfig { headers: None });
    let headers = request.headers.unwrap_or_else(|| HashMap::new());
    let mut swagger = get_swagger(config.url, headers);

    let mut paths = swagger.paths.clone();
    if let Some(mut config_paths) = config.paths {
        let remove = config_paths.remove("__remove");
        rename_paths(&mut paths, config_paths);
        match remove {
            Some(paths_to_remove) => {
                remove_paths(&mut paths, paths_to_remove);
            }
            None => {}
        }
    }
    change_operation_id(&mut paths);
    remove_query_params(&mut paths);

    paths.sort_keys();
    swagger.paths = paths;

    let mut definitions = swagger.definitions.clone();
    if let Some(mut def) = config.definitions {
        if let Some(defs_to_remove) = def.shift_remove("__remove") {
            remove_definitions(&mut definitions, defs_to_remove);
        }
        merge_definitions(&mut definitions, def);
    }
    update_required(&mut definitions);

    remove_unused_definitions(&swagger.paths, &mut definitions);

    definitions.sort_keys();
    swagger.definitions = definitions;

    let used_tags = collect_tags(&swagger.paths);
    swagger.tags.retain(|t| used_tags.contains(&t.name));
    swagger.tags.sort_by_key(|t| t.name.clone());

    write_swagger(config.file, swagger);
}

fn remove_unused_definitions(paths: &Paths, definitions: &mut Definitions) {
    loop {
        let defs = find_usages(&paths, &definitions);

        let rm: Vec<String> = definitions
            .keys()
            .cloned()
            .filter(|k| !defs.contains(k))
            .collect();

        if rm.is_empty() {
            break;
        }

        for key in rm {
            definitions.shift_remove(&key);
        }
    }
}

fn find_usages(paths: &Paths, definitions: &Definitions) -> Vec<String> {
    let mut defs = Vec::new();

    for methods in paths.values() {
        for method in methods.values() {
            let method_obj = method.as_object().unwrap();
            let parameters = method_obj["parameters"].as_array().unwrap();
            let responses = method_obj["responses"].as_object().unwrap();
            for parameter in parameters {
                let parameter_obj = parameter.as_object().unwrap();
                if let Some(schema) = parameter_obj.get("schema") {
                    if let Some(original_ref) = find_original_ref(schema) {
                        insert_unique(&mut defs, original_ref);
                    }
                }
            }
            for response in responses.values() {
                if let Some(schema) = response.get("schema") {
                    if let Some(original_ref) = find_original_ref(schema) {
                        insert_unique(&mut defs, original_ref);
                    }
                }
            }
        }
    }

    for dto in definitions.values() {
        if let Some(properties) = dto.get("properties") {
            for property in properties.as_object().unwrap().values() {
                if let Some(original_ref) = find_original_ref(property) {
                    insert_unique(&mut defs, original_ref);
                }
            }
        }
    }

    defs.sort();
    defs
}

fn find_original_ref(schema_or_property: &JsonValue) -> Option<String> {
    if let Some(obj) = schema_or_property.as_object() {
        if let Some(original_ref) = obj.get("originalRef") {
            return Some(original_ref.as_str().unwrap().to_string());
        }

        match (obj.get("type"), obj.get("items")) {
            (Some(obj_type), Some(obj_items)) => {
                if obj_type.as_str().unwrap() == "array" {
                    let items = obj_items.as_object().unwrap();
                    if let Some(original_ref) = items.get("originalRef") {
                        return Some(original_ref.as_str().unwrap().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    None
}

fn insert_unique<T: PartialEq>(vec: &mut Vec<T>, item: T) {
    if !vec.contains(&item) {
        vec.push(item);
    }
}

fn remove_paths(paths: &mut Paths, paths_to_remove: YamlValue) {
    if let Some(remove_paths) = paths_to_remove.as_sequence() {
        for path in remove_paths {
            let key = path.as_str().unwrap();
            paths.shift_remove(key);
        }
    }
}

fn rename_paths(paths: &mut Paths, paths_to_rename: HashMap<String, YamlValue>) {
    let mut renamed: HashMap<String, String> = HashMap::new();
    for (search, value) in paths_to_rename {
        let re = Regex::new(&search).unwrap();
        let rep = value.as_str().unwrap();
        let matched = paths.keys().filter(|k| re.is_match(k));
        for key in matched {
            renamed.insert(key.to_string(), re.replace(&key, rep).to_string());
        }
    }
    for (from, to) in renamed {
        if let Some(v) = paths.shift_remove(&from) {
            paths.insert(to, v);
        }
    }
}

fn remove_definitions(definitions: &mut Definitions, defs_to_remove: YamlValue) {
    if let Some(remove_defs) = defs_to_remove.as_sequence() {
        for def in remove_defs {
            let key = def.as_str().unwrap();
            definitions.shift_remove(key);
        }
    }
}

fn merge_definitions(definitions: &mut Definitions, defs_to_merge: IndexMap<String, YamlValue>) {
    for (key, value) in defs_to_merge {
        let it = value.as_mapping().unwrap().to_owned();
        let mut definition: IndexMap<String, JsonValue> = IndexMap::new();
        definition.extend(it.iter().map(|(key, value)| {
            (
                key.as_str().unwrap().to_string(),
                serde_json::to_value(value).unwrap(),
            )
        }));
        if let Some(existing) = definitions.shift_remove(&key) {
            let mut merged = existing;
            merge(&mut merged, definition);
            definitions.insert(key, merged);
        } else {
            definitions.insert(key, definition);
        }
    }
}

fn update_required(definitions: &mut Definitions) {
    for (_key, definition) in definitions.iter_mut() {
        if definition.get("required").is_none_or(|value| {
            if let Some(arr) = value.as_array() {
                arr.contains(&JsonValue::String("*".to_string()))
            } else {
                false
            }
        }) {
            if let Some(properties) = definition.get("properties") {
                definition.insert(
                    "required".to_string(),
                    properties
                        .as_object()
                        .unwrap()
                        .keys()
                        .map(|k| k.to_string())
                        .collect::<Vec<_>>()
                        .into(),
                );
            }
        }
        definition.sort_keys();
    }
}

fn remove_query_params(paths: &mut Paths) {
    let mut renamed: HashMap<String, String> = HashMap::new();
    let re = Regex::new("\\{\\?.*\\}").unwrap();
    let matched = paths.keys().filter(|k| re.is_match(k));
    for key in matched {
        renamed.insert(key.to_string(), re.replace(&key, "").to_string());
    }
    for (from, to) in renamed {
        if let Some(v) = paths.shift_remove(&from) {
            paths.insert(to, v);
        }
    }
}

fn change_operation_id(paths: &mut Paths) {
    for (url, path) in paths.iter_mut() {
        for (verb, value) in path.iter_mut() {
            let m_object = value.as_object_mut().unwrap();
            m_object["operationId"] = generate_operation_id(verb, url).into();
        }
    }
}

fn generate_operation_id(verb: &String, url: &String) -> String {
    let mut op: String;
    match verb.as_str() {
        "get" => op = "get".to_string(),
        "post" => op = "create".to_string(),
        "put" => op = "update".to_string(),
        "delete" => op = "delete".to_string(),
        _ => op = "".to_string(),
    }

    let re_delimeter = Regex::new(r"[/ ]+").unwrap();
    let str = re_delimeter
        .split(url)
        .skip(1)
        .map(|slug| {
            let re_path = Regex::new(r"\{(\w+)}").unwrap();
            let captures = re_path.captures(slug);
            match captures {
                Some(m) => {
                    let (_full, [word]) = m.extract();
                    word
                }
                _ => slug,
            }
        })
        .skip(1)
        .map(|s| s.to_case(Case::Pascal))
        .collect::<Vec<_>>()
        .join("");

    op.push_str(&str);

    op
}

fn collect_tags(paths: &Paths) -> HashSet<String> {
    let mut tags = HashSet::new();
    for methods in paths.values() {
        for method in methods.values() {
            let m_object = method.as_object().unwrap();
            for tag in m_object["tags"].as_array().unwrap() {
                tags.insert(tag.as_str().unwrap().to_string());
            }
        }
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swagger::Swagger;

    #[test]
    fn it_find_usages() {
        // Arrange
        let data = r#"{
"swagger": "2.0",
"info": {
    "version": "1",
    "title": "test-api"
},
"host": "",
"basePath": "/",
"tags": [],
"paths": {
    "/api": {
        "get": {
            "parameters": [
                {
                    "name": "query",
                    "in": "query",
                    "type": "string"
                }
            ],
            "responses": {
                "200": {
                    "description": "OK",
                    "schema": {
                        "originalRef": "ResponseDto"
                    }
                }
            }
        },
        "post": {
            "parameters": [],
            "responses": {
                "200": {
                    "description": "OK",
                    "schema": {
                        "type": "array",
                        "items": {
                            "originalRef": "ResponseItemDto"
                        }
                    }
                }
            }
        },
        "delete": {
            "parameters": [
                {
                    "in": "body",
                    "schema": {
                        "originalRef": "ParametersDto"
                    }
                }
            ],
            "responses": {}
        }
    }
},
"definitions": {
    "ResponseDto": {
        "type": "object",
        "required": [],
        "properties": {
            "status": {
                "originalRef": "StatusDto"
            },
            "rows": {
                "type": "array",
                "items": {
                    "originalRef": "ResponseRowDto"
                }
            }
        },
        "title": "ResponseDto"
    },
    "StatusDto": {
        "type": "string"
    },
    "ResponseRowDto": {
        "type": "object"
    },
    "ParametersDto": {
        "type": "object",
        "required": ["id"],
        "properties": {
            "id": {
                "type": "string"
            }
        },
        "title": "ParametersDto"
    }
}
        }"#;
        let swagger: Swagger = serde_json::from_str(data).unwrap();
        let expected = [
            "ParametersDto",
            "ResponseDto",
            "ResponseItemDto",
            "ResponseRowDto",
            "StatusDto",
        ];

        // Act
        let result = find_usages(&swagger.paths, &swagger.definitions);

        // Assert
        assert_eq!(result, expected);
    }
}
