use rust_baml::{
    extraction::BamlExtractor,
    ontology::Ontology,
};

fn ontology() -> Ontology {
    serde_json::from_str(include_str!("../ontology.json")).unwrap()
}

#[test]
fn generates_baml_from_ontology() {
    let ontology = ontology();
    let extractor = BamlExtractor::new(&ontology);

    let schema = extractor.generate_schema();

    assert!(schema.contains("class Person"));
    assert!(schema.contains("class Company"));
    assert!(schema.contains("source_id string"));
    assert!(schema.contains("class ExtractionResult"));
    assert!(schema.contains("person Person[]"));
}
