//! Converts the canonical JSON ontology into a BAML adapter file.
//!
//! This is not the ontology itself. The real source-of-truth is `ontology.json`.
//! The BAML output is a generated AI-facing schema used by the extraction layer.
use serde_json::{Map, Value};
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Default, Debug, Clone)]
pub struct BamlConverter {
    data: Value,
}

impl BamlConverter {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let data: Value = serde_json::from_str(&content)?;
        Ok(Self { data })
    }

    fn ontology(&self) -> (Map<String, Value>, Map<String, Value>) {
        let nodes = self
            .data
            .get("nodes")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_else(Map::new);

        let relationships = self
            .data
            .get("relationships")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_else(Map::new);

        (nodes, relationships)
    }

    fn relationship_descriptions_for_node(&self, node_name: &str) -> Vec<String> {
        let (_, relationships) = self.ontology();
        let mut descriptions = Vec::new();

        for (relationship_name, relationship_data) in &relationships {
            let from = relationship_data
                .get("from")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let to = relationship_data
                .get("to")
                .and_then(Value::as_str)
                .unwrap_or_default();

            if from == node_name {
                descriptions.push(format!("{relationship_name} -> {to}"));
            } else if to == node_name {
                descriptions.push(format!("{relationship_name} <- {from}"));
            }
        }

        descriptions
    }

    pub fn generate(&self) -> String {
        let (nodes, _) = self.ontology();
        let mut out = Vec::new();

        for (node, details) in &nodes {
            let mut properties = details
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_else(Map::new);

            let relationship_description = self.relationship_descriptions_for_node(node.as_str());
            let parent_description = if relationship_description.is_empty() {
                "Stable source identifiers of all parent nodes".to_string()
            } else {
                format!(
                    "Stable source identifiers of all parent nodes via relationships: {}",
                    relationship_description.join(", ")
                )
            };

            for (property_name, property_type, property_description) in [
                ("source_id", "string", "Stable source identifier for deduplication and parent mapping"),
                ("parent_source_ids", "string[]", parent_description.as_str()),
            ] {
                properties.entry(property_name.to_string()).or_insert(serde_json::json!({
                    "type": property_type,
                    "description": property_description
                }));
            }

            out.push(format!("class {node} {{"));

            for (property_name, property_data) in properties {
                let property_type = property_data
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("string");
                let property_description = property_data
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let quoted_description = format!("@description(#\"{property_description}\"#)");
                out.push(format!("  {property_name} {property_type} {quoted_description}"));
            }

            out.push("}\n".to_string());
        }

        out.push("dynamic class Result {".to_string());

        for node in nodes.keys() {
            let field = match node.as_str() {
                "Person" => "person".to_string(),
                "Company" => "company".to_string(),
                "Skill" => "skills".to_string(),
                _ => node.to_ascii_lowercase(),
            };

            out.push(format!("  {field} {node}[]"));
        }

        out.push("}\n".to_string());

        out.join("\n")
    }


}

#[cfg(test)]
mod tests {
    use super::BamlConverter;
    
    #[test]
    fn generates_normalized_entity_shape_for_person_company_and_skills() {
        let converter = BamlConverter::from_file("ontology.json").unwrap();
        let generated = converter.generate();

        assert!(generated.contains("class Person"));
        assert!(!generated.contains("hasskills Skill[]"));
        assert!(!generated.contains("worksat Company"));
        assert!(generated.contains("parent_source_ids string[]"));
        assert!(generated.contains("worksAt -> Company"));
        assert!(generated.contains("hasSkills -> Skill"));
        assert!(generated.contains("person Person[]"));
        assert!(generated.contains("company Company[]"));
        assert!(generated.contains("skills Skill[]"));
        assert!(!generated.contains("skill Skill"));
    }
}
