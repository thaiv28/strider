use gpui::{Hsla, rgb};
/// The categorical palette is shared with lib/categories.ts; unknown names use Theme::muted.
pub fn category_color(name: &str) -> Option<Hsla> {
    Some(
        rgb(match name {
            "Shelter" => 0x2f7e72,
            "Sleep" => 0x4e7b96,
            "Storage" => 0xc2913b,
            "Kitchen" => 0xc25a34,
            "Clothing" => 0x6e8b4f,
            "Electronics" => 0x5b6b84,
            "Safety" => 0xd8a72e,
            "Tools & Repair" => 0x9a6a44,
            "Health & Hygiene" => 0xa65a6e,
            _ => return None,
        })
        .into(),
    )
}
