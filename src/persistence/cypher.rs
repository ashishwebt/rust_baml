use crate::extraction::EntitySchema;
use serde_json::Value;
use std::collections::HashMap;

pub struct CypherAdapter;

impl CypherAdapter {
    fn fmt_value(v: &Value) -> String {
        match v {
            Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            other => format!("\"{}\"", other.to_string().replace('"', "\\\"")),
        }
    }

    #[allow(dead_code)]
    fn record_props_for_schema(schema: &EntitySchema, record: &Value) -> Option<HashMap<String, Value>> {
        // Expect record to be an object with either a single object for the schema name
        // or an array of objects. This helper extracts matching entries.
        if let Value::Object(map) = record {
            if let Some(v) = map.get(&schema.name) {
                if v.is_object() {
                    if let Value::Object(obj) = v {
                        let mut out = HashMap::new();
                        for (k, _) in &schema.properties {
                            if let Some(val) = obj.get(k) {
                                out.insert(k.clone(), val.clone());
                            }
                        }
                        return Some(out);
                    }
                }
            }
        }
        None
    }

    pub fn generate_for_payload(schemas: &[EntitySchema], payload: &Value) -> String {
        let mut parts: Vec<String> = Vec::new();

        for schema in schemas {
            // If payload contains entries for this schema name (case-insensitive
            // and with simple plural variants), create nodes for each matching
            // object or array element.
            if let Value::Object(map) = payload {
                let name_lower = schema.name.to_lowercase();
                let candidates = vec![
                    schema.name.clone(),
                    name_lower.clone(),
                    format!("{}s", name_lower),
                ];

                for key in candidates {
                    if let Some(items) = map.get(&key) {
                        match items {
                            Value::Array(arr) => {
                                for item in arr {
                                    let mut props: Vec<String> = Vec::new();
                                    if let Value::Object(obj) = item {
                                        for (k, _) in &schema.properties {
                                            if let Some(val) = obj.get(k) {
                                                props.push(format!("{}: {}", k, Self::fmt_value(val)));
                                            }
                                        }
                                    }
                                    parts.push(format!("CREATE (:{label} {{ {props} }});", label = schema.name, props = props.join(", ")));
                                }
                            }
                            Value::Object(obj) => {
                                let mut props: Vec<String> = Vec::new();
                                for (k, _) in &schema.properties {
                                    if let Some(val) = obj.get(k) {
                                        props.push(format!("{}: {}", k, Self::fmt_value(val)));
                                    }
                                }
                                parts.push(format!("CREATE (:{label} {{ {props} }});", label = schema.name, props = props.join(", ")));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        parts.join("\n")
    }
}

impl super::PersistenceAdapter for CypherAdapter {
    fn generate_queries(&self, schemas: &[EntitySchema], payload: &Value) -> String {
        Self::generate_for_payload(schemas, payload)
    }
}
