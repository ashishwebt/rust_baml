mod baml_client;
mod extraction;
mod ontology;
mod persistence;

use ontology::{OntologyNormalizer};
use std::fs;
use extraction::{Extractor, JsonFileExtractor,extract_from_ontology};
use persistence::{CypherAdapter, PersistenceAdapter};

fn main() {
    dotenvy::from_filename(".env").ok();

    let cypher_path = "cypher_creation.cypher";
    

    let sample_text = "John Doe is a Senior Software Engineer at Acme Corporation, a healthcare company. \
        He has skills in Python, Machine Learning,  and Rust. \
        His email is john.doe@acme.com and phone is +1-555-123-4567. \
        He lives at 123 Main Street, Springfield, Illinois.";

    let res_value = extract_from_ontology("ontology.json", sample_text)
        .expect("failed to extract from ontology");

    let normalizer = OntologyNormalizer::new();
    let canonical_ontology = normalizer.normalize_extracted(&res_value);
    let canonical_path = "normalized_ontology.json";
    let canonical_text = serde_json::to_string_pretty(&canonical_ontology).expect("failed to pretty-print normalized ontology");
    fs::write(canonical_path, &canonical_text).expect("failed to write normalized ontology to file");

    let extractor = JsonFileExtractor;
    let schemas = extractor.extract(std::path::Path::new("ontology.json")).expect("failed to extract schemas");
    let adapter = CypherAdapter;
    let new_data_layer = adapter.generate_queries(&schemas, &canonical_ontology);
    let cypher_output = format!(" Canonical ontology JSON\n{canonical_text}\n\n// Persistence adapter\n{new_data_layer}");
    fs::write(cypher_path, &cypher_output).expect("failed to write cypher output to file");
    println!("Saved Cypher creation layer to {cypher_path}\n");
    println!("Cypher output:\n{cypher_output}\n");

    // Only normalized JSON and cypher files are persisted.
}
