class Result {
  summary string
  @@dynamic  // Dynamic nodes and edges will be injected here
}

function ExtractInfo(input: string) -> Result {
  client "CustomGemini"
  prompt #"
    Extract structured information from the following text based on the requested schema.

    {{ ctx.output_format }}

    {{ _.role("user") }} {{ input }}
  "#
}