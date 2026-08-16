use serde_json::{Map, Value};
use uuid::Uuid;

#[derive(Debug, Default, Clone)]
pub struct OntologyNormalizer;

impl OntologyNormalizer {
    pub fn new() -> Self {
        Self
    }


    fn canonical_entity_name(raw: &str) -> &str {
        match raw.to_ascii_lowercase().as_str() {
            "person" | "people" => "Person",
            "company" | "companies" => "Company",
            "skill" | "skills" => "Skill",
            _ => raw,
        }
    }

    fn normalize_record(_entity_name: &str, record: &Value) -> Value {
        let mut obj = record.clone();
        if let Value::Object(map) = &mut obj {
            if !map.contains_key("source_id") {
                // generate a UUID v4 for source_id to ensure uniqueness across systems
                let id = Uuid::new_v4().to_string();
                map.insert("source_id".to_string(), Value::String(id));
            }

            if let Some(ids) = map.get("parent_source_ids") {
                if let Value::String(s) = ids {
                    let list = vec![Value::String(s.clone())];
                    map.insert("parent_source_ids".to_string(), Value::Array(list));
                }
            }
        }

        obj
    }

    pub fn normalize_extracted(&self, extracted: &Value) -> Value {
        let mut out = Map::new();

        let object = match extracted {
            Value::Object(obj) => obj,
            _ => return extracted.clone(),
        };

        for (key, value) in object {
            let entity_name = Self::canonical_entity_name(key);
            let records = match value {
                Value::Array(items) => items.iter().cloned().collect::<Vec<_>>(),
                Value::Object(_) => vec![value.clone()],
                _ => continue,
            };

            let normalized = records
                .into_iter()
                .map(|record| Self::normalize_record(entity_name, &record))
                .collect::<Vec<_>>();

            if !normalized.is_empty() {
                out.insert(entity_name.to_string(), Value::Array(normalized));
            }
        }

        Value::Object(out)
    }
}

#[cfg(test)]
mod tests {
    use super::OntologyNormalizer;
    use serde_json::json;

    #[test]
    fn normalizes_raw_extracted_payload_into_canonical_ontology_json() {
        let normalizer = OntologyNormalizer::new();
        let extracted = json!({
            "person": [{
                "name": "John Doe",
                "email": "john.doe@acme.com",
                "parent_source_ids": ["company_1"]
            }],
            "company": [{
                "name": "Acme Corporation",
                "source_id": "company_1"
            }],
            "skills": [{
                "name": "Rust"
            }]
        });

        let normalized = normalizer.normalize_extracted(&extracted);

        assert!(normalized.get("Person").is_some());
        assert!(normalized.get("Company").is_some());
        assert!(normalized.get("Skill").is_some());
        assert!(normalized["Person"][0]["source_id"].is_string());
        assert!(normalized["Person"][0]["parent_source_ids"][0] == "company_1");
    }
}
