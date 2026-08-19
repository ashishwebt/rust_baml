use serde_json::Value;
use std::error::Error;
use std::path::Path;
// Hide BAML-specific wiring behind this function so main.rs doesn't depend on baml_client.
pub fn run_baml_adapter(baml_adapter: &str, input: &str) -> Result<Value, Box<dyn Error>> {
    // Build a TypeBuilder, register the generated adapter, and call the BAML client.
    let tb = crate::baml_client::type_builder::TypeBuilder::new();
    tb.add_baml(baml_adapter)?;

    // Call the generated sync client. Keep the dependency local to this module.
    let res = crate::baml_client::sync_client::B
        .ExtractInfo
        .with_type_builder(&tb)
        .call(input)?;

    let v = serde_json::to_value(&res)?;
    Ok(v)
}

/// Convenience: load ontology JSON, generate the BAML adapter, and run extraction.
pub fn extract_from_ontology(ontology_path: impl AsRef<Path>, input: &str) -> Result<Value, Box<dyn Error>> {
    let converter = crate::extraction::JsonOntologyAdapter::from_file(ontology_path)?;
    let baml_adapter = converter.generate();
    run_baml_adapter(&baml_adapter, input)
}
