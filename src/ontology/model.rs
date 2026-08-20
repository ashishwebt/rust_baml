use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySpec {
    #[serde(rename = "type")]
    pub data_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub description: Option<String>,
    #[serde(default)]
    pub properties: HashMap<String, PropertySpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ontology {
    pub nodes: HashMap<String, Entity>,
    #[serde(default)]
    pub relationships: HashMap<String, Relationship>,
}

#[derive(Debug, Error)]
pub enum OntologyError {
    #[error("ontology contains no nodes")]
    EmptyOntology,

    #[error("invalid entity name: {0}")]
    InvalidEntityName(String),

    #[error("relationship '{relationship}' references unknown entity '{entity}'")]
    UnknownEntity { relationship: String, entity: String },

    #[error("unsupported property type '{data_type}' for {entity}.{property}")]
    UnsupportedPropertyType {
        entity: String,
        property: String,
        data_type: String,
    },
}

impl Ontology {
    pub fn validate(&self) -> Result<(), OntologyError> {
        if self.nodes.is_empty() {
            return Err(OntologyError::EmptyOntology);
        }

        for name in self.nodes.keys() {
            if name.trim().is_empty() {
                return Err(OntologyError::InvalidEntityName(name.clone()));
            }
        }

        for (entity_name, entity) in &self.nodes {
            for (property_name, property) in &entity.properties {
                if !matches!(
                    property.data_type.as_str(),
                    "string" | "integer" | "float" | "boolean" |
                    "string[]" | "integer[]" | "float[]" | "boolean[]"
                ) {
                    return Err(OntologyError::UnsupportedPropertyType {
                        entity: entity_name.clone(),
                        property: property_name.clone(),
                        data_type: property.data_type.clone(),
                    });
                }
            }
        }

        for (name, relationship) in &self.relationships {
            for entity in [&relationship.from, &relationship.to] {
                if !self.nodes.contains_key(entity) {
                    return Err(OntologyError::UnknownEntity {
                        relationship: name.clone(),
                        entity: entity.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}
