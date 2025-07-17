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
        if let Some(paths_to_remove) = config_paths.remove("__remove") {
            remove_paths(&mut paths, paths_to_remove);
        }
        rename_paths(&mut paths, config_paths);
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
    definitions.sort_keys();
    swagger.definitions = definitions;

    let used_tags = collect_tags(&swagger.paths);
    swagger.tags.retain(|t| used_tags.contains(&t.name));
    swagger.tags.sort_by_key(|t| t.name.clone());

    write_swagger(config.file, swagger);
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
