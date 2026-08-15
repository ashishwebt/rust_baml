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

    pub fn generate(&self) -> String {
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

        let mut out = Vec::new();

        for (node, details) in &nodes {
            let description = details
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");

            out.push(format!("/// {description}\nclass {node} {{"));

            if let Some(properties) = details.get("properties").and_then(Value::as_object) {
                for (property_name, property_data) in properties {
                    let property_type = property_data
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("string");
                    let property_description = property_data
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or("");

                    out.push(format!(
                        "  /// {property_description}\n  {property_name} {property_type}"
                    ));
                }
            }

            for (relationship_name, relationship_data) in &relationships {
                if relationship_data
                    .get("from")
                    .and_then(Value::as_str)
                    .map(|value| value == node)
                    .unwrap_or(false)
                {
                    let target = relationship_data
                        .get("to")
                        .and_then(Value::as_str)
                        .unwrap_or("Unknown");
                    let is_array = if relationship_name
                        .to_ascii_lowercase()
                        .contains("skill")
                        || relationship_name.to_ascii_lowercase().contains("has")
                    {
                        "[]"
                    } else {
                        ""
                    };

                    out.push(format!(
                        "  {} {}{}",
                        relationship_name.to_ascii_lowercase(),
                        target,
                        is_array
                    ));
                }
            }

            out.push("}\n".to_string());
        }

        out.push("// Connect new classes to the @@dynamic Result class".to_string());
        out.push("dynamic class Result {".to_string());

        for node in nodes.keys() {
            out.push(format!("  {} {}", node.to_ascii_lowercase(), node));
        }

        out.push("}\n".to_string());

        out.join("\n")
    }
}
