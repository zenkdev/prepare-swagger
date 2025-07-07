mod config;
mod merge_map;
mod swagger;
mod yaml;

use convert_case::{Case, Casing};
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::config::Schema;
use crate::config::get_config;
use crate::merge_map::merge;
use crate::swagger::Paths;
use crate::swagger::get_swagger;
use crate::yaml::save_yaml;

fn main() {
    let usage = format!(
        "Usage: {} [path_to_config_file]",
        std::env::args().next().unwrap()
    );
    let path = std::env::args().nth(1).expect(&usage);

    let config = get_config(path);
    let schemas = config.schemas;

    for schema in schemas {
        process_schema(schema);
    }
}

fn process_schema(schema: Schema) {
    let multienv = schema.multienv.unwrap_or_else(|| "default".to_string());
    let mut swagger = get_swagger(schema.url, multienv);

    let mut paths = swagger.paths.clone();
    if let Some(p) = schema.paths {
        remove_paths(&mut paths, p.__remove);
        rename_paths(&mut paths, p.__rename);
    }
    change_operation_id(&mut paths);
    remove_query_params(&mut paths);
    paths.sort_keys();
    swagger.paths = paths;

    let mut definitions = swagger.definitions.clone();
    if let Some(def) = schema.definitions {
        let remove_definitions = def.__remove.unwrap_or_default();
        for definition in remove_definitions {
            definitions.shift_remove(&definition);
        }
        let add_definitions = def.__add.unwrap_or_default();
        for (key, definition) in add_definitions {
            definitions.insert(key, definition);
        }
        let override_definitions = def.__override.unwrap_or_default();
        for (key, definition) in override_definitions {
            if let Some(existing) = definitions.shift_remove(&key) {
                let mut merged = existing;
                merge(&mut merged, definition);
                definitions.insert(key, merged);
            }
        }
    }

    for (_key, definition) in definitions.iter_mut() {
        if !definition.contains_key("required") {
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
    definitions.sort_keys();
    swagger.definitions = definitions;

    let used_tags = collect_tags(&swagger.paths);
    swagger.tags.retain(|t| used_tags.contains(&t.name));
    swagger.tags.sort_by_key(|t| t.name.clone());

    save_yaml(schema.file, swagger);
}

fn remove_paths(paths: &mut Paths, paths_to_remove: Option<Vec<String>>) {
    let remove_paths = paths_to_remove.unwrap_or_default();
    for path in remove_paths {
        paths.shift_remove(&path);
    }
}

fn rename_paths(paths: &mut Paths, paths_to_rename: Option<HashMap<String, String>>) {
    if let Some(rename) = paths_to_rename {
        let mut renamed: HashMap<String, String> = HashMap::new();
        for (search, replace) in rename {
            let re = Regex::new(&search).unwrap();
            let matched = paths.keys().filter(|k| re.is_match(k));
            for key in matched {
                renamed.insert(key.to_string(), re.replace(&key, &replace).to_string());
            }
        }
        for (from, to) in renamed {
            if let Some(v) = paths.shift_remove(&from) {
                paths.insert(to, v);
            }
        }
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
