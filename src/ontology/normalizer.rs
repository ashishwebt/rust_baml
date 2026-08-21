use serde_json::{Map, Value};
use std::collections::HashMap;
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

        // Maps the extractor's logical source_id to one UUID.
        //
        // Example:
        // "company_1" -> "550e8400-e29b-41d4-a716-446655440000"
        let mut source_id_map: HashMap<String, String> = HashMap::new();

        let mut output = Map::new();

        for (name, records) in input {
            let records = match records {
                Value::Array(items) => items
                    .iter()
                    .map(|record| Self::normalize_record(record, &mut source_id_map))
                    .collect(),

                Value::Object(_) => {
                    vec![Self::normalize_record(records, &mut source_id_map)]
                }

                _ => continue,
            };

            output.insert(name.clone(), Value::Array(records));
        }

        Value::Object(output)
    }

    fn normalize_record(
        value: &Value,
        source_id_map: &mut HashMap<String, String>,
    ) -> Value {
        let Value::Object(object) = value else {
            return value.clone();
        };

        let mut result = object.clone();

        // Replace source_id with its UUID.
        if let Some(Value::String(source_id)) = object.get("source_id") {
            let uuid = Self::get_or_create_uuid(source_id, source_id_map);

            result.insert(
                "source_id".to_string(),
                Value::String(uuid),
            );
        } else {
            // If extraction did not provide a source_id,
            // create a new UUID for this record.
            result.insert(
                "source_id".to_string(),
                Value::String(Uuid::new_v4().to_string()),
            );
        }

        // Replace every parent_source_id with the UUID
        // associated with that logical source ID.
        if let Some(Value::Array(parent_ids)) = object.get("parent_source_ids") {
            let normalized_parents = parent_ids
                .iter()
                .filter_map(|parent_id| {
                    let Value::String(parent_id) = parent_id else {
                        return None;
                    };

                    Some(Value::String(
                        Self::get_or_create_uuid(parent_id, source_id_map),
                    ))
                })
                .collect();

            result.insert(
                "parent_source_ids".to_string(),
                Value::Array(normalized_parents),
            );
        } else {
            result.insert(
                "parent_source_ids".to_string(),
                Value::Array(Vec::new()),
            );
        }

        Value::Object(result)
    }

    fn get_or_create_uuid(
        source_id: &str,
        source_id_map: &mut HashMap<String, String>,
    ) -> String {
        source_id_map
            .entry(source_id.to_string())
            .or_insert_with(|| Uuid::new_v4().to_string())
            .clone()
    }
}