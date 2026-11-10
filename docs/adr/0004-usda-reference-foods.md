# Use bundled USDA reference foods for ingredients

Ingredients default to bundled USDA reference foods, with calories and measured volume weight supplied by the same record. Density is never inferred from an ingredient name. Packaged products are an opt-in search.

## Consequences

When no density is available, users confirm a USDA reference match or enter a measured weight for a volume. Stored density records its USDA, label, or manual provenance; reference lookup does not call external food APIs.
