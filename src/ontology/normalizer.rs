use serde_json::{Map, Value};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct OntologyNormalizer;

impl OntologyNormalizer {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize(&self, value: &Value) -> Value {
        let Value::Object(input) = value else {
            return value.clone();
        };

        let mut output = Map::new();

        for (name, records) in input {
            let records = match records {
                Value::Array(items) => items
                    .iter()
                    .map(Self::normalize_record)
                    .collect(),
                Value::Object(_) => vec![Self::normalize_record(records)],
                _ => continue,
            };

            output.insert(name.clone(), Value::Array(records));
        }

        Value::Object(output)
    }

    fn normalize_record(value: &Value) -> Value {
        let Value::Object(object) = value else {
            return value.clone();
        };

        let mut result: Map<String, Value> = object.clone();

        result.entry("source_id".to_string()).or_insert_with(|| {
            Value::String(Uuid::new_v4().to_string())
        });


        result.entry("parent_source_ids".to_string()).or_insert_with(|| {
            Value::Array(Vec::new())
        });

        Value::Object(result)
    }
}
