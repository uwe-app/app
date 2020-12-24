use serde_json::Value;

// Look up a dot-delimited path in an object.
pub fn find_path<S: AsRef<str>>(needle: S, doc: &Value) -> Value {
    #[allow(unused_assignments)]
    let mut parent = Value::Null;

    let parts = needle
        .as_ref()
        .split(".")
        .map(|p| p.to_string())
        .enumerate()
        .collect::<Vec<_>>();

    match doc {
        Value::Object(ref _map) => {
            let mut current: &Value = doc;
            for (i, part) in parts.iter() {
                if *i == parts.len() - 1 {
                    return find_field(&part, current);
                } else {
                    parent = find_field(&part, current);
                    if let Value::Null = parent {
                        break;
                    }
                    current = &parent;
                }
            }
        }
        _ => {}
    }
    Value::Null
}

/*
pub fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Object(ref _map) => return true,
        Value::Array(ref _list) => return true,
        Value::String(ref s) => return s.len() > 0,
        Value::Bool(ref b) => return *b,
        Value::Number(ref n) => {
            if n.is_i64() {
                return n.as_i64().unwrap() != 0;
            } else if n.is_u64() {
                return n.as_u64().unwrap() != 0;
            } else if n.is_f64() {
                return n.as_f64().unwrap() != 0.0;
            }
        }
        _ => {}
    }
    false
}

// Lookup a value and determine whether it is truthy.
pub fn truthy<S: AsRef<str>>(needle: S, doc: &Value) -> bool {
    let val = find_path(needle, doc);
    is_truthy(&val)
}
*/

// Look up a field in an array or object.
pub fn find_field<S: AsRef<str>>(field: S, parent: &Value) -> Value {
    match parent {
        Value::Object(ref map) => {
            if let Some(val) = map.get(field.as_ref()) {
                return val.clone();
            }
        }
        Value::Array(ref list) => {
            if let Ok(index) = field.as_ref().parse::<usize>() {
                if !list.is_empty() && index < list.len() {
                    return list[index].clone();
                }
            }
        }
        _ => {}
    }
    Value::Null
}

// Sort a list of values by path lookup.
pub fn sort<S: AsRef<str>>(needle: S, values: &mut Vec<&Value>) {
    values.sort_by(|a, b| {
        let s1 = find_path(needle.as_ref(), a).to_string();
        let s2 = find_path(needle.as_ref(), b).to_string();
        s1.partial_cmp(&s2).unwrap()
    });
}
