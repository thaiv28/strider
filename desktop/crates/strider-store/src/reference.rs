//! Offline port of lib/usda-reference.ts. The index is parsed once per process.
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceFood {
    pub id: i64,
    pub name: String,
    pub reference_name: String,
    pub density_g_ml: Option<f64>,
    pub kcal_per_100g: Option<f64>,
}
struct Food {
    hit: ReferenceFood,
    tokens: Vec<String>,
    first: Vec<String>,
    segments: Vec<Vec<String>>,
    sr: bool,
}
const ALIASES: &[(&str, &str)] = &[
    ("rolled oats", "oats regular quick dry"),
    ("quick oats", "oats regular quick dry"),
    ("powdered milk", "milk dry"),
    ("dry milk", "milk dry"),
    ("milk powder", "milk dry"),
    ("evoo", "oil olive"),
    ("white sugar", "sugars granulated"),
    ("red pepper flakes", "pepper red cayenne"),
    ("cavatappi", "pasta dry enriched"),
    ("rotini", "pasta dry enriched"),
    ("penne", "pasta dry enriched"),
    ("fusilli", "pasta dry enriched"),
    ("farfalle", "pasta dry enriched"),
    ("garlic powder", "spices garlic powder"),
    ("onion powder", "spices onion powder"),
];
fn words(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            if w.len() > 3 {
                w.strip_suffix('s').unwrap_or(w)
            } else {
                w
            }
            .to_owned()
        })
        .collect()
}
fn hidden(description: &str) -> bool {
    let first = description
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("");
    if matches!(first, "Babyfood" | "Restaurant") {
        return true;
    }
    if description.strip_prefix("Fast ").is_some_and(|s| {
        ["Foods", "Food", "foods", "food"].iter().any(|p| {
            s.strip_prefix(p).is_some_and(|tail| {
                tail.chars()
                    .next()
                    .is_none_or(|c| !c.is_alphanumeric() && c != '_')
            })
        })
    }) {
        return true;
    }
    description.split_once(',').is_some_and(|(prefix, _)| {
        prefix.chars().count() >= 4
            && prefix
                .chars()
                .all(|c| c.is_ascii_uppercase() || "'’&. -".contains(c))
    })
}
fn foods() -> &'static [Food] {
    static FOODS: OnceLock<Vec<Food>> = OnceLock::new();
    FOODS.get_or_init(|| {
        let data: Vec<Vec<serde_json::Value>> =
            serde_json::from_str(include_str!("../../../../lib/usda-reference-data.json"))
                .expect("bundled USDA index must be valid");
        data.into_iter()
            .filter_map(|r| {
                let description = r.get(1)?.as_str()?;
                if hidden(description) {
                    return None;
                }
                Some(Food {
                    hit: ReferenceFood {
                        id: r.first()?.as_i64()?,
                        name: description.into(),
                        reference_name: description.into(),
                        density_g_ml: r.get(2)?.as_f64(),
                        kcal_per_100g: r.get(3)?.as_f64(),
                    },
                    tokens: words(description),
                    first: words(description.split(',').next().unwrap_or("")),
                    segments: description.split(',').map(words).collect(),
                    sr: r.get(5)?.as_str() == Some("sr"),
                })
            })
            .collect()
    })
}
pub fn get_reference_food(id: i64) -> Option<ReferenceFood> {
    foods()
        .iter()
        .find(|f| f.hit.id == id)
        .map(|f| f.hit.clone())
}
fn matches(token: &str, word: &str) -> bool {
    token == word || (token.len() >= 3 && word.starts_with(token))
}
fn less_likely(description: &str) -> bool {
    let lower = description.to_lowercase();
    [
        "cooked",
        "prepared",
        "reconstituted",
        "made with",
        "glutinous",
        "restaurant",
        "steamed",
        "bowl",
    ]
    .iter()
    .any(|phrase| {
        lower.match_indices(phrase).any(|(start, _)| {
            let boundary = |c: char| !c.is_ascii_alphanumeric() && c != '_';
            lower[..start].chars().next_back().is_none_or(boundary)
                && lower[start + phrase.len()..]
                    .chars()
                    .next()
                    .is_none_or(boundary)
                && (*phrase != "reconstituted" || !lower[..start].ends_with("not "))
        })
    })
}
pub fn search_reference_foods(
    query: &str,
    limit: usize,
    min_matched_share: f64,
) -> Vec<ReferenceFood> {
    let q = query.trim();
    if q.len() < 2 {
        return vec![];
    }
    let lower = q.to_lowercase();
    let mut expanded = match lower.as_str() {
        "sugar" => "sugars granulated",
        "rice" => "rice white long grain raw enriched",
        _ => &lower,
    }
    .to_owned();
    let mut aliased = expanded != lower;
    for (phrase, replacement) in ALIASES {
        let mut next = String::new();
        let mut end = 0;
        for (start, _) in expanded.match_indices(phrase) {
            let after = start + phrase.len();
            let boundary = |c: char| !c.is_ascii_alphanumeric() && c != '_';
            if expanded[..start].chars().next_back().is_none_or(boundary)
                && expanded[after..].chars().next().is_none_or(boundary)
            {
                next.push_str(&expanded[end..start]);
                next.push_str(replacement);
                end = after;
                aliased = true;
            }
        }
        next.push_str(&expanded[end..]);
        expanded = next;
    }
    let tokens = words(&expanded);
    if tokens.is_empty() {
        return vec![];
    }
    let mut ranked: Vec<_> = foods()
        .iter()
        .filter_map(|f| {
            let matched: Vec<_> = tokens
                .iter()
                .filter(|t| f.tokens.iter().any(|w| matches(t, w)))
                .collect();
            if matched.is_empty()
                || (matched.len() as f64) < tokens.len() as f64 * min_matched_share
            {
                return None;
            }
            let rank = [
                tokens.len() - matched.len(),
                matched.iter().filter(|t| !f.tokens.contains(t)).count(),
                matched
                    .iter()
                    .filter(|t| {
                        !f.segments
                            .iter()
                            .take(2)
                            .any(|s| s.iter().any(|w| matches(t, w)))
                    })
                    .count(),
                f.first
                    .iter()
                    .filter(|w| !tokens.iter().any(|t| matches(t, w)))
                    .count(),
                usize::from(less_likely(&f.hit.reference_name)),
                usize::from(f.hit.density_g_ml.is_none()),
                usize::from(!f.sr),
                f.tokens.len(),
            ];
            Some((f, matched.len(), rank))
        })
        .collect();
    ranked.sort_by(|a, b| {
        a.2.cmp(&b.2)
            .then_with(|| {
                a.0.hit
                    .reference_name
                    .to_lowercase()
                    .cmp(&b.0.hit.reference_name.to_lowercase())
            })
            .then(a.0.hit.id.cmp(&b.0.hit.id))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(f, count, _)| {
            let mut hit = f.hit.clone();
            if if aliased {
                count == tokens.len()
            } else {
                tokens.iter().all(|t| f.tokens.contains(t))
            } {
                let mut chars = q.chars();
                hit.name = chars
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default();
            }
            hit
        })
        .collect()
}
pub fn match_reference_foods(name: &str, limit: usize) -> Vec<ReferenceFood> {
    search_reference_foods(name, foods().len(), 0.)
        .into_iter()
        .filter(|f| f.density_g_ml.is_some())
        .take(limit)
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staples_and_aliases() {
        let sugar = search_reference_foods("sugar", 12, 0.5);
        assert!(
            sugar[0]
                .reference_name
                .to_lowercase()
                .starts_with("sugars, granulated")
        );
        assert_eq!(sugar[0].name, "Sugar");
        assert_eq!(
            search_reference_foods("rolled oats", 12, 0.5)[0].id,
            search_reference_foods("oats regular quick dry", 12, 0.5)[0].id
        );
        assert!(search_reference_foods("x", 12, 0.5).is_empty());
    }
    // IDs produced by the unchanged TypeScript over the bundled index (no server/database).
    #[test]
    fn typescript_reference_search_goldens() {
        for (query, expected) in [
            ("sugar", [169655, 2710258, 170675]),
            ("rolled oats", [173904, 173905, 171675]),
            ("butter", [173410, 173430, 173411]),
            ("evoo", [171413, 2710186, 167737]),
            ("brown sugar", [168833, 2710260, 172801]),
            ("milk powder", [170876, 173454, 171272]),
            ("rice", [168877, 168879, 169707]),
        ] {
            assert_eq!(
                search_reference_foods(query, 3, 0.5)
                    .iter()
                    .map(|f| f.id)
                    .collect::<Vec<_>>(),
                expected,
                "{query}"
            );
        }
        assert!(!less_likely("Milk, not reconstituted"));
        assert!(less_likely("Milk, reconstituted"));
        assert!(!less_likely("Bowls"));
    }
    #[test]
    fn whole_words_and_hidden_entries() {
        let results = search_reference_foods("butter", usize::MAX, 0.5);
        let whole = results
            .iter()
            .position(|f| words(&f.reference_name).contains(&"butter".into()))
            .expect("butter");
        let prefix = results
            .iter()
            .position(|f| f.reference_name.to_lowercase().contains("butterbur"))
            .expect("butterbur");
        assert!(whole < prefix);
        assert!(foods().iter().all(|f| !hidden(&f.hit.reference_name)));
        assert!(
            search_reference_foods("Restaurant", 100, 0.)
                .iter()
                .all(|f| !f.reference_name.starts_with("Restaurant"))
        );
        assert!(
            match_reference_foods("olive oil", 4)
                .iter()
                .all(|f| f.density_g_ml.is_some())
        );
    }
}
