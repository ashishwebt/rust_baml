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

    pub fn generate_cypher_creation_layer(&self) -> String {
        let (nodes, relationships) = self.ontology();
        let mut statements = Vec::new();

        for (node_name, details) in &nodes {
            let mut properties = details
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_else(Map::new);

            for key in ["source_id", "parent_source_id"] {
                properties.entry(key.to_string()).or_insert(Value::Object(Map::new()));
            }

            let property_names = properties.keys().cloned().collect::<Vec<_>>();

            if property_names.is_empty() {
                statements.push(format!("MERGE (n:{node_name});"));
                continue;
            }

            let property_assignments = property_names
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

    fn source_id_for(node_name: &str, value: &Value) -> Option<String> {
        let obj = value.as_object()?;

        let explicit_id = ["source_id", "sourceId", "slug", "id"]
            .iter()
            .find_map(|key| obj.get(*key).and_then(Value::as_str))
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);

        explicit_id.or_else(|| {
            obj.get("name")
                .and_then(Value::as_str)
                .map(|name| format!("{node_name}:{}", Self::slugify(name)))
        })
    }

    fn parent_source_ids_for(_node_name: &str, value: &Value) -> Vec<String> {
        let obj = match value.as_object() {
            Some(obj) => obj,
            None => return Vec::new(),
        };

        let from_array = obj
            .get("parent_source_ids")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !from_array.is_empty() {
            return from_array;
        }

        obj.get("parent_source_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(|value| vec![value.to_string()])
            .unwrap_or_default()
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect()
    }

    fn relationship_key(name: &str) -> String {
        name.to_ascii_lowercase().replace('_', "")
    }

    fn property_pairs_for_node(node_name: &str, item: &Value, details: &Value) -> Vec<String> {
        let properties = item.as_object().cloned().unwrap_or_else(Map::new);

        let mut pairs = details
            .get("properties")
            .and_then(Value::as_object)
            .map(|expected_props| {
                expected_props
                    .keys()
                    .filter_map(|key| {
                        if matches!(properties.get(key), Some(Value::Object(_) | Value::Array(_))) {
                            return None;
                        }

                        let value = properties.get(key)?;
                        let text = match value {
                            Value::String(s) if key == "source_id" && s.trim().is_empty() => return None,
                            Value::String(s) if key == "parent_source_id" && s.trim().is_empty() => return None,
                            _ => format!("{key}: {}", Self::cypher_literal(value)),
                        };
                        Some(text)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if let Some(source_id) = Self::source_id_for(node_name, item) {
            pairs.insert(0, format!("source_id: \"{}\"", source_id.replace('"', "\\\"")));
        }

        let parent_source_ids = Self::parent_source_ids_for(node_name, item);
        if !parent_source_ids.is_empty() {
            let values = parent_source_ids
                .iter()
                .map(|id| format!("\"{}\"", id.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(", ");
            pairs.insert(1, format!("parent_source_ids: [{values}]"));
        }

        pairs
    }

    #[allow(dead_code)]
    pub fn generate_cypher_from_extracted(&self, extracted: &Value) -> String {
        let (nodes, relationships) = self.ontology();
        let mut statements = Vec::new();

        for (node_name, details) in &nodes {
            let root_keys = [node_name.to_ascii_lowercase(), node_name.to_string()];
            let candidate = root_keys.iter().find_map(|key| extracted.get(key));

            let items = match candidate {
                Some(Value::Array(values)) => values.iter().cloned().collect::<Vec<_>>(),
                Some(Value::Object(obj)) => vec![Value::Object(obj.clone())],
                _ => Vec::new(),
            };

            for item in items {
                let property_pairs = Self::property_pairs_for_node(node_name, &item, details);

                if property_pairs.is_empty() {
                    statements.push(format!("MERGE (n:{node_name}));"));
                } else {
                    let properties_text = property_pairs.join(", ");
                    statements.push(format!("MERGE (n:{node_name} {{{properties_text}}});"));
                }
            }
        }

        for (relationship_name, relationship_data) in &relationships {
            let from = relationship_data.get("from").and_then(Value::as_str).unwrap_or_default();
            let to = relationship_data.get("to").and_then(Value::as_str).unwrap_or_default();
            let rel_type = Self::cypher_relationship_name(relationship_name);

            if from.is_empty() || to.is_empty() {
                continue;
            }

            let from_key = from.to_ascii_lowercase();
            let relationship_key = Self::relationship_key(relationship_name);

            if let Some(parent_obj) = extracted.get(&from_key).and_then(Value::as_object) {
                if let Some(value) = parent_obj.get(&relationship_key) {
                    let source_var = if from == "Person" { "p" } else if from == "Company" { "c" } else { "a" };
                    let target_var = if to == "Skill" { "s" } else if to == "Company" { "c" } else { "t" };

                    for target in Self::as_value_list(value) {
                        if let (Some(from_source), Some(to_source)) = (
                            Self::source_id_for(from, &Value::Object(parent_obj.clone())),
                            Self::source_id_for(to, &target),
                        ) {
                            statements.push(format!(
                                "MATCH ({source_var}:{from} {{source_id: \"{}\"}}), ({target_var}:{to} {{source_id: \"{}\"}}) MERGE ({source_var})-[:{rel_type}]->({target_var});",
                                from_source.replace('"', "\\\""),
                                to_source.replace('"', "\\\"")
                            ));
                        }
                    }
                }
            }

            let direct_match = format!("MATCH (a:{from}), (b:{to}) MERGE (a)-[:{rel_type}]->(b);");
            statements.push(direct_match);
        }

        if statements.is_empty() {
            statements.push("// No extracted nodes or relationships were available.".to_string());
        }

        statements.join("\n")
    }

    fn as_value_list(value: &Value) -> Vec<Value> {
        match value {
            Value::Array(items) => items.clone(),
            Value::Object(_) => vec![value.clone()],
            _ => Vec::new(),
        }
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

    #[test]
    fn generates_cypher_from_source_linked_objects() {
        let converter = BamlConverter::from_file("ontology.json").unwrap();
        let extracted = json!({
            "company": {
                "source_id": "company:acme-corp",
                "name": "Acme Corporation"
            },
            "person": {
                "source_id": "person:john-doe",
                "name": "John Doe",
                "email": "john.doe@acme.com",
                "worksat": {
                    "source_id": "company:acme-corp",
                    "name": "Acme Corporation"
                },
                "hasskills": [
                    { "source_id": "skill:python", "name": "Python" },
                    { "source_id": "skill:rust", "name": "Rust" }
                ]
            }
        });

        let cypher = converter.generate_cypher_from_extracted(&extracted);

        assert!(cypher.contains("MERGE (n:Person {source_id: \"person:john-doe\""));
        assert!(cypher.contains("MERGE (n:Company {source_id: \"company:acme-corp\""));
        assert!(cypher.contains("MATCH (p:Person {source_id: \"person:john-doe\"}), (s:Skill {source_id: \"skill:python\"}) MERGE (p)-[:HAS_SKILLS]->(s);"));
        assert!(cypher.contains("MATCH (p:Person {source_id: \"person:john-doe\"}), (c:Company {source_id: \"company:acme-corp\"}) MERGE (p)-[:WORKS_AT]->(c);"));
    }

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
