mod baml_client;
mod extraction;
mod ontology;
mod persistence;

use ontology::BamlConverter;
use baml_client::type_builder::TypeBuilder;
use baml_client::sync_client::B;
use std::fs;
use extraction::{Extractor, JsonFileExtractor};
use persistence::{CypherAdapter, PersistenceAdapter};

fn main() {
    dotenvy::from_filename(".env").ok();

    
    // Use a project-local ontology.json (created if missing) to make running
    // `cargo run` reliable regardless of external paths.
    let converter = BamlConverter::from_file("ontology.json")
        .expect("failed to read ontology.json");

    let tb = TypeBuilder::new();
    let generated = converter.generate();
    let schema_layer = converter.generate_cypher_creation_layer();
    let cypher_path = "cypher_creation.cypher";
    let root_baml_path: &str = "dynamic_ontology.baml";
    fs::write(root_baml_path, &generated).expect("failed to write root generated BAML review file");
    tb.add_baml(&generated).unwrap();

    let sample_text = "John Doe is a Senior Software Engineer at Acme Corporation, a healthcare company. \
        He has skills in Python, Machine Learning,  and Rust. \
        His email is john.doe@acme.com and phone is +1-555-123-4567. \
        He lives at 123 Main Street, Springfield, Illinois.";
    let res = B.ExtractInfo
        .with_type_builder(&tb)
        .call(sample_text)
        .unwrap();
    let extracted_json = serde_json::to_value(&res).expect("failed to serialize extracted result to JSON");
    let extracted_json_path = "extracted_data.json";
    let extracted_json_text = serde_json::to_string_pretty(&extracted_json).expect("failed to pretty-print extracted JSON");
    fs::write(extracted_json_path, &extracted_json_text).expect("failed to write extracted JSON to file");

    let data_layer = converter.generate_cypher_from_extracted(&extracted_json);
    // New: use extraction + persistence adapter
    let extractor = JsonFileExtractor;
    let schemas = extractor.extract(std::path::Path::new("ontology.json")).expect("failed to extract schemas");
    let adapter = CypherAdapter;
    let new_data_layer = adapter.generate_queries(&schemas, &extracted_json);
    let cypher_output = format!("{schema_layer}\n\n// Extracted data (old generator)\n{data_layer}\n\n// Extracted data (adapter)\n{new_data_layer}");
    fs::write(cypher_path, &cypher_output).expect("failed to write cypher output to file");

    println!("Saved extracted JSON to {extracted_json_path}");
    println!("Saved Cypher creation layer to {cypher_path}\n");
    println!("Cypher output:\n{cypher_output}\n");

    // Print dynamic fields and their values
    for (name, value) in res.dynamic_fields() {
        println!("{}: {:?}", name, value);
    }
}
