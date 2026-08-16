use serde_json::{Map, Value};

#[derive(Debug, Default, Clone)]
pub struct OntologyNormalizer;

impl OntologyNormalizer {
    pub fn new() -> Self {
        Self
    }

    fn slugify(value: &str) -> String {
        let mut slug = String::new();
        for ch in value.to_ascii_lowercase().chars() {
            if ch.is_ascii_alphanumeric() {
                slug.push(ch);
            } else if !slug.is_empty() && !slug.ends_with('-') {
                slug.push('-');
            }
        }
        slug.trim_matches('-').to_string()
    }

    fn canonical_entity_name(raw: &str) -> &str {
        match raw.to_ascii_lowercase().as_str() {
            "person" | "people" => "Person",
            "company" | "companies" => "Company",
            "skill" | "skills" => "Skill",
            _ => raw,
        }
    }

    fn normalize_record(entity_name: &str, record: &Value) -> Value {
        let mut obj = record.clone();
        if let Value::Object(map) = &mut obj {
            if !map.contains_key("source_id") {
                if let Some(name) = map.get("name").and_then(Value::as_str) {
                    let id = format!("{}:{}", entity_name.to_ascii_lowercase(), Self::slugify(name));
                    map.insert("source_id".to_string(), Value::String(id));
                }
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
