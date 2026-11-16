//! Gear inventory totals and the wishlist's hypothetical loadout projection.
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightClass {
    Base,
    Worn,
    Consumable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Current,
    Retired,
    Wishlist,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GearRow {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub weight_g: Option<i64>,
    pub quantity: i64,
    pub price_cents: Option<i64>,
    pub default_weight_class: WeightClass,
    pub status: Status,
    pub replaces: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Subtotal {
    pub base_g: i64,
    pub consumable_g: i64,
    pub worn_g: i64,
    pub base_items: Vec<SliceItem>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SliceItem {
    pub name: String,
    pub grams: i64,
    pub quantity: i64,
}
pub type Membership = BTreeMap<String, WeightClass>;
/// A kit's *per-unit* weight, before multiplying the kit's own quantity.
pub fn kit_weight(components: impl IntoIterator<Item = (i64, i64)>) -> i64 {
    components.into_iter().map(|(g, q)| g * q).sum()
}
/// None selects all current library gear and its default classes. Some selects
/// only current members and uses the loadout's class, independent of defaults.
pub fn category_subtotals(
    rows: &[GearRow],
    membership: Option<&Membership>,
) -> BTreeMap<Option<String>, Subtotal> {
    let mut result = BTreeMap::<Option<String>, Subtotal>::new();
    for r in rows {
        let subtotal = result.entry(r.category.clone()).or_default();
        if r.status != Status::Current {
            continue;
        }
        let class = if let Some(m) = membership {
            let Some(class) = m.get(&r.id) else { continue };
            *class
        } else {
            r.default_weight_class
        };
        let grams = r.weight_g.unwrap_or(0) * r.quantity;
        match class {
            WeightClass::Base => {
                subtotal.base_g += grams;
                subtotal.base_items.push(SliceItem {
                    name: r.name.clone(),
                    grams,
                    quantity: r.quantity,
                });
            }
            WeightClass::Worn => subtotal.worn_g += grams,
            WeightClass::Consumable => subtotal.consumable_g += grams,
        }
    }
    result
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WishlistProjection {
    pub projected_base_g: i64,
    pub baseline_g: i64,
    pub total_cents: i64,
    pub count: usize,
    pub price_known: bool,
}
/// Exact wishlist-frame rules: all-gear includes every wishlist row. A loadout
/// assumes inclusion when *any* replaced id is a member (even worn); an explicit
/// per-item override wins. New weight uses the wishlist item's default class.
/// Retired weights use current items only and their effective loadout class.
/// Replaced ids are deduplicated across purchases, so an old item retires once.
/// Unknown prices make price_known false without discarding known prices.
pub fn wishlist_projection(
    rows: &[GearRow],
    membership: Option<&Membership>,
    overrides: &BTreeMap<String, bool>,
) -> WishlistProjection {
    let current: BTreeMap<_, _> = rows
        .iter()
        .filter(|r| r.status == Status::Current)
        .map(|r| (&r.id, r))
        .collect();
    let is_base = |r: &GearRow| {
        membership.map_or(r.default_weight_class == WeightClass::Base, |m| {
            m.get(&r.id) == Some(&WeightClass::Base)
        })
    };
    let mut p = WishlistProjection {
        price_known: true,
        ..Default::default()
    };
    p.baseline_g = current
        .values()
        .filter(|r| is_base(r))
        .map(|r| r.weight_g.unwrap_or(0) * r.quantity)
        .sum();
    let mut retired = BTreeSet::new();
    let mut add = 0;
    for r in rows.iter().filter(|r| r.status == Status::Wishlist) {
        let included = membership.is_none_or(|m| {
            overrides
                .get(&r.id)
                .copied()
                .unwrap_or_else(|| r.replaces.iter().any(|id| m.contains_key(id)))
        });
        if !included {
            continue;
        }
        p.count += 1;
        if r.default_weight_class == WeightClass::Base {
            add += r.weight_g.unwrap_or(0) * r.quantity;
        }
        if let Some(price) = r.price_cents {
            p.total_cents += price * r.quantity;
        } else {
            p.price_known = false;
        }
        for id in &r.replaces {
            if membership.is_none_or(|m| m.contains_key(id)) {
                retired.insert(id);
            }
        }
    }
    let remove: i64 = retired
        .into_iter()
        .filter_map(|id| current.get(id))
        .filter(|r| is_base(r))
        .map(|r| r.weight_g.unwrap_or(0) * r.quantity)
        .sum();
    p.projected_base_g = (p.baseline_g + add - remove).max(0);
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(id: &str, status: Status, g: i64, class: WeightClass, replaces: &[&str]) -> GearRow {
        GearRow {
            id: id.into(),
            name: id.into(),
            category: None,
            weight_g: Some(g),
            quantity: 1,
            price_cents: Some(100),
            default_weight_class: class,
            status,
            replaces: replaces.iter().map(|s| s.to_string()).collect(),
        }
    }
    #[test]
    fn membership_classes_and_kit() {
        let rows = [
            row("a", Status::Current, 100, WeightClass::Worn, &[]),
            row("b", Status::Retired, 900, WeightClass::Base, &[]),
        ];
        let m = BTreeMap::from([("a".into(), WeightClass::Base)]);
        assert_eq!(category_subtotals(&rows, Some(&m))[&None].base_g, 100);
        assert_eq!(category_subtotals(&rows, None)[&None].worn_g, 100);
        assert_eq!(kit_weight([(10, 2), (15, 3)]), 65);
    }
    #[test]
    fn replacement_dedup_and_explicit_overrides() {
        let rows = [
            row("a", Status::Current, 100, WeightClass::Base, &[]),
            row("w", Status::Current, 200, WeightClass::Worn, &[]),
            row("x", Status::Wishlist, 20, WeightClass::Base, &["a"]),
            row("y", Status::Wishlist, 30, WeightClass::Base, &["a"]),
            row("z", Status::Wishlist, 10, WeightClass::Base, &["w"]),
        ];
        let m = BTreeMap::from([
            ("a".into(), WeightClass::Base),
            ("w".into(), WeightClass::Worn),
        ]);
        let p = wishlist_projection(&rows, Some(&m), &BTreeMap::new());
        assert_eq!(p.projected_base_g, 60);
        assert_eq!(p.count, 3);
        let p = wishlist_projection(
            &rows,
            Some(&m),
            &BTreeMap::from([("x".into(), false), ("y".into(), false)]),
        );
        assert_eq!(p.projected_base_g, 110);
        assert_eq!(p.count, 1);
    }
    #[test]
    fn forced_new_item_and_unknown_price() {
        let mut wish = row("new", Status::Wishlist, 25, WeightClass::Base, &[]);
        wish.price_cents = None;
        wish.quantity = 2;
        let membership = Membership::new();
        assert_eq!(
            wishlist_projection(&[wish.clone()], Some(&membership), &BTreeMap::new()).count,
            0
        );
        let p = wishlist_projection(
            &[wish.clone()],
            Some(&membership),
            &BTreeMap::from([("new".into(), true)]),
        );
        assert_eq!(p.projected_base_g, 50);
        assert!(!p.price_known);
        // All-gear ignores explicit loadout overrides.
        assert_eq!(
            wishlist_projection(&[wish], None, &BTreeMap::from([("new".into(), false)])).count,
            1
        );
    }
}
