# rust_baml

Ontology-driven extraction with Rust and BAML.

## Structure

```text
ontology.json
      |
      v
   Ontology
      |
   validate
      |
      v
 BamlExtractor
   |       |
   |       +--> ontology -> BAML schema
   |
   +----------> BAML -> LLM -> structured JSON
                    |
                    v
                Normalizer
                    |
                    v
              CypherAdapter
                    |
                    v
                  Neo4j
```

`generation` and `extraction` are intentionally merged.

`BamlExtractor` owns:

```rust
generate_schema()
extract()
```

The ontology remains the source of truth. Persistence remains separate.

The actual generated BAML client invocation should be wired into
`BamlExtractor::extract()` using the generated client already used by
the project.
