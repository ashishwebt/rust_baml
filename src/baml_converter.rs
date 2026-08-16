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

    fn cypher_relationship_name(raw_name: &str) -> String {
        let mut out = String::new();
        let chars: Vec<char> = raw_name.chars().collect();

        for (index, ch) in chars.iter().enumerate() {
            if ch.is_ascii_uppercase() {
                if index > 0 && !chars[index - 1].is_ascii_uppercase() {
                    out.push('_');
                }
                out.push(ch.to_ascii_uppercase());
            } else {
                out.push(ch.to_ascii_uppercase());
            }
        }

        out
    }

    pub fn generate(&self) -> String {
        let (nodes, relationships) = self.ontology();
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

    pub fn generate_cypher_creation_layer(&self) -> String {
        let (nodes, relationships) = self.ontology();
        let mut statements = Vec::new();

        for (node_name, details) in &nodes {
            let properties = details
                .get("properties")
                .and_then(Value::as_object)
                .map(|map| map.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();

            if properties.is_empty() {
                statements.push(format!("MERGE (n:{node_name});"));
                continue;
            }

            let property_assignments = properties
                .iter()
                .map(|property_name| format!("{property_name}: ${}", property_name.to_ascii_lowercase()))
                .collect::<Vec<_>>()
                .join(", ");

            statements.push(format!("MERGE (n:{node_name} {{{property_assignments}}});"));
        }

        for (relationship_name, relationship_data) in &relationships {
            let from = relationship_data
                .get("from")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let to = relationship_data
                .get("to")
                .and_then(Value::as_str)
                .unwrap_or_default();

            if !from.is_empty() && !to.is_empty() {
                let rel_type = Self::cypher_relationship_name(relationship_name);
                statements.push(format!(
                    "MATCH (a:{from}), (b:{to}) MERGE (a)-[:{rel_type}]->(b);"
                ));
            }
        }

        statements.join("\n")
    }

    #[allow(dead_code)]
    pub fn generate_cypher_from_extracted(&self, extracted: &Value) -> String {
        let (nodes, relationships) = self.ontology();
        let mut statements = Vec::new();

        for (node_name, details) in &nodes {
            let root_keys = [node_name.to_ascii_lowercase(), node_name.to_string()];
            let candidate = root_keys
                .iter()
                .find_map(|key| extracted.get(key));

            let items = match candidate {
                Some(Value::Array(values)) => values.iter().cloned().collect::<Vec<_>>(),
                Some(Value::Object(obj)) => vec![Value::Object(obj.clone())],
                _ => Vec::new(),
            };

            for item in items {
                let properties = item
                    .as_object()
                    .cloned()
                    .unwrap_or_else(Map::new);
                let property_pairs = details
                    .get("properties")
                    .and_then(Value::as_object)
                    .map(|expected_props| {
                        expected_props
                            .keys()
                            .filter_map(|key| {
                                properties
                                    .get(key)
                                    .map(|value| format!("{key}: {}", Self::cypher_literal(value)))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if property_pairs.is_empty() {
                    statements.push(format!("MERGE (n:{node_name}));"));
                } else {
                    let properties_text = property_pairs.join(", ");
                    statements.push(format!("MERGE (n:{node_name} {{{properties_text}}});"));
                }
            }
        }

        for (relationship_name, relationship_data) in &relationships {
            let from = relationship_data
                .get("from")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let to = relationship_data
                .get("to")
                .and_then(Value::as_str)
                .unwrap_or_default();

            let rel_type = Self::cypher_relationship_name(relationship_name);
            if !from.is_empty() && !to.is_empty() {
                statements.push(format!(
                    "MATCH (a:{from}), (b:{to}) MERGE (a)-[:{rel_type}]->(b);"
                ));
            }
        }

        if statements.is_empty() {
            statements.push("// No extracted nodes or relationships were available.".to_string());
        }

        statements.join("\n")
    }

    #[allow(dead_code)]
    fn cypher_literal(value: &Value) -> String {
        match value {
            Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(items) => {
                let inner = items
                    .iter()
                    .map(Self::cypher_literal)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            Value::Object(_) => "{}".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BamlConverter;
    use serde_json::json;

    #[test]
    fn generates_cypher_layer_from_ontology() {
        let converter = BamlConverter::from_file("ontology.json").unwrap();
        let cypher = converter.generate_cypher_creation_layer();

        assert!(cypher.contains("MERGE (n:Person"));
        assert!(cypher.contains("MERGE (n:Company"));
        assert!(cypher.contains("MATCH (a:Person), (b:Company) MERGE (a)-[:WORKS_AT]->(b);"));
        assert!(cypher.contains("MATCH (a:Person), (b:Skill) MERGE (a)-[:HAS_SKILLS]->(b);"));
    }

    #[test]
    fn generates_cypher_from_extracted_values() {
        let converter = BamlConverter::from_file("ontology.json").unwrap();
        let extracted = json!({
            "person": [{
                "name": "John Doe",
                "email": "john.doe@acme.com"
            }],
            "company": [{
                "name": "Acme Corp"
            }],
            "skill": [{
                "name": "Rust"
            }]
        });

        let cypher = converter.generate_cypher_from_extracted(&extracted);

        assert!(cypher.contains("MERGE (n:Person"));
        assert!(cypher.contains("name: \"John Doe\""));
        assert!(cypher.contains("email: \"john.doe@acme.com\""));
        assert!(cypher.contains("MERGE (n:Company"));
        assert!(cypher.contains("name: \"Acme Corp\""));
        assert!(cypher.contains("MATCH (a:Person), (b:Company) MERGE (a)-[:WORKS_AT]->(b);"));
    }
}
