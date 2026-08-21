use serde_json::Value;
use std::collections::HashMap;

pub struct CypherAdapter;

impl CypherAdapter {
    pub fn new() -> Self {
        Self
    }

    fn records(value: &Value) -> Vec<&Value> {
        match value {
            Value::Array(values) => values.iter().collect(),
            Value::Object(_) => vec![value],
            _ => Vec::new(),
        }
    }

    fn format_value(value: &Value) -> String {
        match value {
            Value::String(value) => format!("\"{}\"", value.replace('"', "\\\"")),
            Value::Number(value) => value.to_string(),
            Value::Bool(value) => value.to_string(),
            Value::Null => "null".into(),
            other => format!("\"{}\"", other.to_string().replace('"', "\\\"")),
        }
    }

    fn relationship_type(child: &str, parent: &str) -> &'static str {
        match (child, parent) {
            ("Person", "Company") => "WORKS_AT",
            ("Person", "Skill") => "HAS_SKILL",
            _ => "RELATED_TO",
        }
    }
}

impl Default for CypherAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl super::PersistenceAdapter for CypherAdapter {
    fn generate_queries(&self, payload: &Value) -> String {
        let Value::Object(entities) = payload else {
            return String::new();
        };

        let mut ids = HashMap::new();

        for (label, records) in entities {
            for record in Self::records(records) {
                if let Some(id) = record.get("source_id").and_then(Value::as_str) {
                    ids.insert(id.to_string(), label.clone());
                }
            }
        }

        let mut output = Vec::new();

        for (label, records) in entities {
            for record in Self::records(records) {
                let Some(object) = record.as_object() else {
                    continue;
                };

                let Some(source_id) = object.get("source_id").and_then(Value::as_str) else {
                    continue;
                };

                output.push(format!(
                    "MERGE (n:{label} {{ source_id: \"{source_id}\" }});"
                ));

                let properties: Vec<String> = object
                    .iter()
                    .filter(|(key, _)| *key != "source_id" && *key != "parent_source_ids")
                    .map(|(key, value)| format!("{key}: {}", Self::format_value(value)))
                    .collect();

                if !properties.is_empty() {
                    output.push(format!("SET n += {{ {} }};", properties.join(", ")));
                }
            }
        }

        for (label, records) in entities {
            for record in Self::records(records) {
                let Some(object) = record.as_object() else {
                    continue;
                };

                let Some(source_id) = object.get("source_id").and_then(Value::as_str) else {
                    continue;
                };

                let Some(Value::Array(parents)) = object.get("parent_source_ids") else {
                    continue;
                };

                for parent in parents {
                    let Some(parent_id) = parent.as_str() else {
                        continue;
                    };

                    let Some(parent_label) = ids.get(parent_id) else {
                        continue;
                    };

                    let relationship = Self::relationship_type(label, parent_label);

                    output.push(format!(
                        "MATCH (a:{label} {{ source_id: \"{source_id}\" }}),
                        (b:{parent_label} {{ source_id: \"{parent_id}\" }})                          MERGE (a)-[:{relationship}]->(b);"
                    ));
                }
            }
        }

        output.join("\n")
    }
}
