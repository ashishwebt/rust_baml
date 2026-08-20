use rust_baml::{extraction::BamlExtractor, ontology::Ontology, persistence::CypherAdapter};
use std::fs;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let text = fs::read_to_string("ontology.json")?;
    let ontology: Ontology = serde_json::from_str(&text)?;
    ontology.validate()?;

    let extractor = BamlExtractor::new(&ontology);

    println!("Ontology is valid.");
    println!("Entities: {}", ontology.nodes.len());
    println!("Relationships: {}", ontology.relationships.len());
    println!();
    println!("{}", extractor.generate_schema());

    let sample_text = "John Doe is a Senior Software Engineer at Acme Corporation, a healthcare company. \
    He has skills in Python, Machine Learning,  and Rust. \
    His email is john.doe@acme.com and phone is +1-555-123-4567. \
    He lives at 123 Main Street, Springfield, Illinois.";
    let dy_extractor = extractor.extract(sample_text).expect("failed to extract schemas");
    let cyper_gen = CypherAdapter::new();
    let query = cyper_gen.generate(&dy_extractor);
    println!("{:?}",query );

    Ok(())
}
