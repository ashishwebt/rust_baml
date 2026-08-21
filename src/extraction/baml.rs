use crate::ontology::{Entity, Ontology};
use serde_json::Value;
use std::error::Error;

pub struct BamlExtractor<'a> {
    ontology: &'a Ontology,
}

impl<'a> BamlExtractor<'a> {
    pub fn new(ontology: &'a Ontology) -> Self {
        Self { ontology }
    }

    pub fn generate_schema(&self) -> String {
        let mut output = String::new();

        for (name, entity) in &self.ontology.nodes {
            Self::generate_entity(&mut output, name, entity);
        }

        output.push_str("dynamic class Result {\n");

        for name in self.ontology.nodes.keys() {
            output.push_str(&format!("  {} {}[]\n", Self::field_name(name), name));
        }

        output.push_str("}\n");
        output
    }

    fn generate_entity(output: &mut String, name: &str, entity: &Entity) {
        output.push_str(&format!("class {name} {{\n"));

        for (property_name, property) in &entity.properties {
            output.push_str(&format!(
                "  {property_name} {}",
                Self::map_type(&property.data_type)
            ));

            if let Some(description) = &property.description {
                output.push_str(&format!(" @description(#\"{}\"#)", description));
            }
            output.push('\n');
        }

        output.push_str("  source_id string\n");
        output.push_str("  parent_source_ids string[]\n");
        output.push_str("}\n\n");
    }

    fn map_type(data_type: &str) -> &str {
        match data_type {
            "integer" => "int",
            "float" => "float",
            "boolean" => "bool",
            "integer[]" => "int[]",
            "float[]" => "float[]",
            "boolean[]" => "bool[]",
            "string[]" => "string[]",
            _ => "string",
        }
    }

    fn field_name(name: &str) -> String {
        let mut chars = name.chars();

        match chars.next() {
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    }
}

impl<'a> super::Extractor for BamlExtractor<'a> {
    
    fn extract(&self, text: &str) -> Result<Value, Box<dyn Error>> {
        let schema = self.generate_schema();

        let tb = crate::baml_client::type_builder::TypeBuilder::new();
        tb.add_baml(&schema)?;

        let res = crate::baml_client::sync_client::B
            .ExtractInfo
            .with_type_builder(&tb)
            .call(text)?;

        let v = serde_json::to_value(&res)?;
        Ok(v)
    }
}
