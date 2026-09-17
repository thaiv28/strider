// Categorical palette for the base-weight breakdown. Muted, earthy mid-tones
// that read on both the warm-ivory light surface and the dark surface.
export const CATEGORY_COLOR: Record<string, string> = {
  Shelter: "#2f7e72",
  Sleep: "#4e7b96",
  Storage: "#c2913b",
  Kitchen: "#c25a34",
  Clothing: "#6e8b4f",
  Electronics: "#5b6b84",
  Safety: "#d8a72e",
  "Tools & Repair": "#9a6a44",
  "Health & Hygiene": "#a65a6e",
};

export const categoryColor = (name: string) => CATEGORY_COLOR[name] ?? "var(--muted)";

// Synthetic category for items with no categoryId. Negative so it never
// collides with a real serial id; moveGear maps it back to a null categoryId.
// Lives here (client-safe, no db import) so client components can reference it.
export const OTHER_CATEGORY_ID = -1;
export const OTHER_CATEGORY_NAME = "Other";
