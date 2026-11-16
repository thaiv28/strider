use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFilter {
    #[default]
    Current,
    Retired,
    All,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GearFilter {
    pub status: StatusFilter,
    pub query: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub weight: Grams,
    pub quantity: i32,
    pub notes: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearRow {
    pub id: String,
    pub name: String,
    pub weight: Option<Grams>,
    pub quantity: i32,
    pub class: String,
    pub status: String,
    pub is_kit: bool,
    pub components: Vec<Component>,
    pub price_cents: Option<Cents>,
    pub product_url: Option<String>,
    pub notes: Option<String>,
    pub category_id: Option<String>,
    pub slot_name: Option<String>,
    pub replaces: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: Option<String>,
    pub name: String,
    pub rows: Vec<GearRow>,
    pub base: Grams,
    pub consumable: Grams,
    pub worn: Grams,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Totals {
    pub base: Grams,
    pub worn: Grams,
    pub consumable: Grams,
    pub items: usize,
    pub by_category: Vec<CategorySlice>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceItem {
    pub name: String,
    pub g: Grams,
    pub quantity: i32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorySlice {
    pub name: String,
    pub g: Grams,
    pub items: Vec<SliceItem>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryOption {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearView {
    pub groups: Vec<Group>,
    pub wishlist: Vec<GearRow>,
    pub category_options: Vec<CategoryOption>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadoutRow {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub sort_order: i32,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadoutData {
    pub loadouts: Vec<LoadoutRow>,
    pub membership: BTreeMap<String, BTreeMap<String, String>>,
    pub wishlist_overrides: BTreeMap<String, BTreeMap<String, bool>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngredientRow {
    pub id: String,
    pub input: IngredientInput,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MealIngredientRow {
    pub id: String,
    pub ingredient_id: Option<String>,
    pub name: String,
    pub kcal_per_100g: Option<Dec>,
    pub density_g_ml: Option<Dec>,
    pub entered_amount: Option<Dec>,
    pub entered_unit: Option<String>,
    pub amount_g: i32,
    pub sort_order: i32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MealRow {
    pub id: String,
    pub name: String,
    pub base_servings: i32,
    pub meal_type: Option<String>,
    pub is_hot: bool,
    pub water_ml: Option<i32>,
    pub notes: Option<String>,
    pub items: Vec<MealIngredientRow>,
    pub total_g: i64,
    pub total_kcal: i64,
}
fn maybe_int(r: &Row, k: &str) -> Option<i32> {
    match r.get(k) {
        Some(Value::Integer(v)) => Some(*v as i32),
        _ => None,
    }
}
fn sum(rows: &[GearRow], class: &str) -> Result<Grams> {
    let total: i64 = rows
        .iter()
        .filter(|r| r.class == class)
        .map(|r| i64::from(r.weight.unwrap_or_default().0) * i64::from(r.quantity))
        .sum();
    i32::try_from(total)
        .map(Grams)
        .map_err(|_| Error::Invalid("Weight subtotal overflow".into()))
}
impl Store {
    pub fn gear_view(&self) -> Result<GearView> {
        let db = &self.connection;
        let mut cats = rows(db, "gear_category")?;
        cats.sort_by_key(|r| (n(r, "sort_order"), s(r, "sync_id").to_string()));
        let comps = rows(db, "gear_component")?;
        let replacements = rows(db, "wishlist_replacement")?;
        let mut raw = rows(db, "gear_item")?;
        raw.sort_by_key(|r| {
            (
                n(r, "sort_order"),
                s(r, "name").to_string(),
                s(r, "sync_id").to_string(),
            )
        });
        let gear: Vec<_> = raw
            .iter()
            .map(|r| {
                let key = s(r, "sync_id");
                GearRow {
                    id: key.into(),
                    name: s(r, "name").into(),
                    weight: maybe_int(r, "weight_g").map(Grams),
                    quantity: n(r, "quantity") as i32,
                    class: s(r, "default_weight_class").into(),
                    status: s(r, "status").into(),
                    is_kit: n(r, "is_kit") != 0,
                    price_cents: maybe_int(r, "price_cents").map(Cents),
                    product_url: opt(r, "product_url"),
                    notes: opt(r, "notes"),
                    category_id: opt(r, "category_id"),
                    slot_name: cats
                        .iter()
                        .find(|c| Some(s(c, "sync_id")) == opt(r, "category_id").as_deref())
                        .map(|c| s(c, "name").to_string()),
                    components: comps
                        .iter()
                        .filter(|c| s(c, "gear_item_id") == key)
                        .map(|c| Component {
                            id: s(c, "sync_id").into(),
                            name: s(c, "name").into(),
                            weight: Grams(n(c, "weight_g") as i32),
                            quantity: n(c, "quantity") as i32,
                            notes: opt(c, "notes"),
                        })
                        .collect(),
                    replaces: replacements
                        .iter()
                        .filter(|r| s(r, "wishlist_item_id") == key)
                        .map(|r| s(r, "replaces_item_id").into())
                        .collect(),
                }
            })
            .collect();
        let options = cats
            .iter()
            .map(|c| CategoryOption {
                id: s(c, "sync_id").into(),
                name: s(c, "name").into(),
            })
            .collect();
        let mut categories: Vec<_> = cats
            .iter()
            .map(|c| (Some(s(c, "sync_id").to_string()), s(c, "name").to_string()))
            .collect();
        categories.push((None, "Other".into()));
        let mut groups = vec![];
        for (id, name) in categories {
            let rows: Vec<_> = gear
                .iter()
                .filter(|r| r.category_id == id)
                .cloned()
                .collect();
            if !rows.is_empty() {
                let current: Vec<_> = rows
                    .iter()
                    .filter(|r| r.status == "current")
                    .cloned()
                    .collect();
                groups.push(Group {
                    id,
                    name,
                    base: sum(&current, "base")?,
                    worn: sum(&current, "worn")?,
                    consumable: sum(&current, "consumable")?,
                    rows,
                });
            }
        }
        Ok(GearView {
            groups,
            wishlist: gear
                .into_iter()
                .filter(|r| r.status == "wishlist")
                .collect(),
            category_options: options,
        })
    }
    pub fn gear_groups(&self, filter: &GearFilter) -> Result<Vec<Group>> {
        let query = filter.query.to_lowercase();
        let mut out = vec![];
        for mut g in self.gear_view()?.groups {
            let matches = g.name.to_lowercase().contains(&query);
            g.rows.retain(|r| {
                let status = match filter.status {
                    StatusFilter::Current => r.status == "current",
                    StatusFilter::Retired => r.status == "retired",
                    StatusFilter::All => r.status != "wishlist",
                };
                status && (matches || r.name.to_lowercase().contains(&query))
            });
            if !g.rows.is_empty() {
                g.base = sum(&g.rows, "base")?;
                g.worn = sum(&g.rows, "worn")?;
                g.consumable = sum(&g.rows, "consumable")?;
                out.push(g);
            }
        }
        Ok(out)
    }
    pub fn loadout_data(&self) -> Result<LoadoutData> {
        let mut out = LoadoutData::default();
        for r in rows(&self.connection, "loadout")? {
            out.loadouts.push(LoadoutRow {
                id: s(&r, "sync_id").into(),
                name: s(&r, "name").into(),
                is_default: n(&r, "is_default") != 0,
                sort_order: n(&r, "sort_order") as i32,
            });
            out.membership
                .insert(s(&r, "sync_id").into(), BTreeMap::new());
        }
        out.loadouts.sort_by_key(|l| (l.sort_order, l.id.clone()));
        for r in rows(&self.connection, "loadout_item")? {
            out.membership
                .entry(s(&r, "loadout_id").into())
                .or_default()
                .insert(s(&r, "gear_item_id").into(), s(&r, "weight_class").into());
        }
        for r in rows(&self.connection, "wishlist_loadout")? {
            out.wishlist_overrides
                .entry(s(&r, "wishlist_item_id").into())
                .or_default()
                .insert(s(&r, "loadout_id").into(), n(&r, "included") != 0);
        }
        Ok(out)
    }
    pub fn totals(&self) -> Result<Totals> {
        let data = self.loadout_data()?;
        let chosen = data
            .loadouts
            .iter()
            .find(|l| l.is_default)
            .or_else(|| data.loadouts.first());
        let membership = chosen.and_then(|l| data.membership.get(&l.id));
        let mut selected = vec![];
        let mut inventory_count = 0;
        let mut by_category: Vec<CategorySlice> = vec![];
        for group in self.gear_groups(&GearFilter::default())? {
            let mut slice = CategorySlice {
                name: group.name.clone(),
                g: Grams(0),
                items: vec![],
            };
            for mut row in group.rows {
                inventory_count += 1;
                if let Some(m) = membership {
                    if let Some(c) = m.get(&row.id) {
                        row.class = c.clone();
                        if c == "base" {
                            add_slice(&mut slice, &row)?;
                        }
                        selected.push(row);
                    }
                } else {
                    if row.class == "base" {
                        add_slice(&mut slice, &row)?;
                    }
                    selected.push(row);
                }
            }
            if membership.is_some() {
                // getDefaultBaseBreakdown merges same-named categories in a loadout.
                if slice.items.is_empty() {
                    continue;
                }
                if let Some(existing) = by_category.iter_mut().find(|c| c.name == slice.name) {
                    existing.g.0 = existing
                        .g
                        .0
                        .checked_add(slice.g.0)
                        .ok_or_else(|| Error::Invalid("Weight overflow".into()))?;
                    existing.items.extend(slice.items);
                } else {
                    by_category.push(slice);
                }
            } else if slice.g.0 > 0 {
                by_category.push(slice);
            }
        }
        by_category.retain(|c| c.g.0 > 0);
        Ok(Totals {
            base: sum(&selected, "base")?,
            worn: sum(&selected, "worn")?,
            consumable: sum(&selected, "consumable")?,
            items: inventory_count,
            by_category,
        })
    }
    pub fn ingredients(&self) -> Result<Vec<IngredientRow>> {
        let mut out = vec![];
        for r in rows(&self.connection, "ingredient")? {
            out.push(IngredientRow {
                id: s(&r, "sync_id").into(),
                input: IngredientInput {
                    name: s(&r, "name").into(),
                    kcal_per_100g: dec(&r, "kcal_per_100g")?,
                    density_g_ml: dec(&r, "density_g_ml")?,
                    density_source: opt(&r, "density_source"),
                    reference_name: opt(&r, "reference_name"),
                    default_serving_g: maybe_int(&r, "default_serving_g"),
                    category: opt(&r, "category"),
                    source: opt(&r, "source"),
                    source_id: opt(&r, "source_id"),
                    notes: opt(&r, "notes"),
                },
            });
        }
        out.sort_by(|a, b| a.input.name.cmp(&b.input.name).then(a.id.cmp(&b.id)));
        Ok(out)
    }
    pub fn meals(&self) -> Result<Vec<MealRow>> {
        let lines = rows(&self.connection, "meal_ingredient")?;
        let mut out = vec![];
        for r in rows(&self.connection, "meal")? {
            let key = s(&r, "sync_id");
            let mut items = vec![];
            for l in lines.iter().filter(|l| s(l, "meal_id") == key) {
                items.push(MealIngredientRow {
                    id: s(l, "sync_id").into(),
                    ingredient_id: opt(l, "ingredient_id"),
                    name: s(l, "snapshot_name").into(),
                    kcal_per_100g: dec(l, "snapshot_kcal_per_100g")?,
                    density_g_ml: dec(l, "snapshot_density_g_ml")?,
                    entered_amount: dec(l, "entered_amount")?,
                    entered_unit: opt(l, "entered_unit"),
                    amount_g: n(l, "amount_g") as i32,
                    sort_order: n(l, "sort_order") as i32,
                });
            }
            items.sort_by_key(|i| (i.sort_order, i.id.clone()));
            let total_g = items.iter().map(|i| i64::from(i.amount_g)).sum();
            let total_kcal = items
                .iter()
                .map(|i| {
                    strider_core::food::kcal_for_grams(
                        i.kcal_per_100g.map(Dec::as_f64),
                        f64::from(i.amount_g),
                    )
                })
                .sum();
            out.push(MealRow {
                id: key.into(),
                name: s(&r, "name").into(),
                base_servings: n(&r, "base_servings") as i32,
                meal_type: opt(&r, "meal_type"),
                is_hot: n(&r, "is_hot") != 0,
                water_ml: maybe_int(&r, "water_ml"),
                notes: opt(&r, "notes"),
                items,
                total_g,
                total_kcal,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(out)
    }
}

fn add_slice(slice: &mut CategorySlice, row: &GearRow) -> Result<()> {
    let g = i64::from(row.weight.unwrap_or_default().0) * i64::from(row.quantity);
    let g = i32::try_from(g).map_err(|_| Error::Invalid("Weight overflow".into()))?;
    slice.g = Grams(
        slice
            .g
            .0
            .checked_add(g)
            .ok_or_else(|| Error::Invalid("Weight overflow".into()))?,
    );
    slice.items.push(SliceItem {
        name: row.name.clone(),
        g: Grams(g),
        quantity: row.quantity,
    });
    Ok(())
}
