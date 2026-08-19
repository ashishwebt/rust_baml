use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySpec {
    #[serde(rename = "type")]
    pub r#type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchema {
    pub name: String,
    pub properties: HashMap<String, PropertySpec>,
}

pub trait Extractor {
    fn extract(&self, ontology_path: &Path) -> Result<Vec<EntitySchema>, Box<dyn std::error::Error>>;
}

pub struct JsonFileExtractor;

impl Extractor for JsonFileExtractor {
    fn extract(&self, ontology_path: &Path) -> Result<Vec<EntitySchema>, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(ontology_path)?;
        let v: Value = serde_json::from_str(&text)?;
        let mut out: Vec<EntitySchema> = Vec::new();

        if let Some(nodes) = v.get("nodes").and_then(|n| n.as_object()) {
            for (name, nodev) in nodes {
                let mut props: HashMap<String, PropertySpec> = HashMap::new();
                if let Some(propsv) = nodev.get("properties").and_then(|p| p.as_object()) {
                    for (pname, pspec) in propsv {
                        let ptype = pspec
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("string")
                            .to_string();
                        let desc = pspec.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());
                        props.insert(pname.clone(), PropertySpec { r#type: ptype, description: desc });
                    }
                }
                out.push(EntitySchema { name: name.clone(), properties: props });
            }
        }

        Ok(out)
    }
}

pub mod baml_runner;
pub use baml_runner::extract_from_ontology;
pub mod baml_converter;