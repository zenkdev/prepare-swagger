use indexmap::{IndexMap, map::Entry};
use serde_json::Value;

pub fn merge(left: &mut IndexMap<String, Value>, right: IndexMap<String, Value>) {
    for (k, v) in right {
        match left.entry(k) {
            Entry::Occupied(mut existing) => match existing.get_mut() {
                Value::Array(arr) => {
                    arr.append(v.to_owned().as_array_mut().unwrap());
                }
                Value::Object(obj) => {
                    let mut scr = v.to_owned();
                    let src_obj = scr.as_object_mut().unwrap();
                    if let Some(remove) = src_obj.remove("__remove") {
                        for k in remove.as_array().unwrap() {
                            obj.remove(k.as_str().unwrap());
                        }
                    }
                    obj.extend(src_obj.to_owned());
                }
                _ => {
                    existing.insert(v);
                }
            },
            Entry::Vacant(empty) => {
                empty.insert(v);
            }
        }
    }
}
