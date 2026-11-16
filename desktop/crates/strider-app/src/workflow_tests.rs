use super::{AddGear, AppView, Entry, gear, input, menus};
use gpui::{AppContext, Context, Entity, TestAppContext, VisualTestContext, Window, WindowOptions};
use gpui_component::Root;
use gpui_component::input::InputState;
use std::collections::BTreeMap;
use strider_core::oz_to_g;
use strider_store::{Command, Store};

fn headless(cx: &mut TestAppContext) -> (Entity<AppView>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        strider_ui::initialize(cx);
        menus(cx);
    });
    let mut store = Store::in_memory().expect("memory");
    strider_store::seed(&mut store, 24).expect("fixture");
    let mut view = None;
    let window = cx.update(|cx| {
        cx.open_window(WindowOptions::default(), |w, cx| {
            let root = cx.new(|cx| AppView::new(store, w, cx));
            view = Some(root.clone());
            cx.new(|cx| Root::new(root, w, cx))
        })
        .expect("headless window")
    });
    (
        view.expect("view"),
        VisualTestContext::from_window(window.into(), cx),
    )
}
fn type_text(state: &Entity<InputState>, text: &str, w: &mut Window, cx: &mut Context<AppView>) {
    state.update(cx, |s, cx| s.set_value(text.to_owned(), w, cx));
}
#[gpui::test]
fn gear_dialog_kit_membership_loadout_wishlist_and_history(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    cx.dispatch_action(AddGear);
    cx.update(|w, cx| {
        view.update(cx, |g, cx| {
            let e = g.editor.as_ref().expect("Add gear action opens dialog");
            type_text(&e.name, "Headless shelter", w, cx);
            type_text(&e.weight, "10", w, cx);
            g.save(w, cx);
            assert!(g.editor.is_none());
            let row = g
                .all_gear
                .iter()
                .find(|r| r.name == "Headless shelter")
                .expect("store reload")
                .clone();
            assert!(
                g.entries
                    .iter()
                    .any(|e| matches!(e,Entry::Item{row:r,..} if r.id==row.id))
            );
            g.edit(row.id.clone(), w, cx);
            let e = g.editor.as_mut().expect("edit");
            e.is_kit = true;
            e.components.push(gear::dialog::ComponentDraft {
                name: input("Fly", false, w, cx),
                weight: input("2", false, w, cx),
                quantity: input("2", false, w, cx),
                notes: None,
            });
            g.save(w, cx);
            assert!(g.editor.is_none());
            let kit = g.all_gear.iter().find(|r| r.id == row.id).expect("kit");
            assert_eq!(kit.weight, Some(oz_to_g(2.) * 2));
            g.open_manager(false, w, cx);
            // Use the same command and store-derived rows as the inline manager.
            g.command(
                Command::CreateLoadout {
                    name: "Headless loadout".into(),
                },
                cx,
            );
            let loadout = g
                .loadouts
                .loadouts
                .iter()
                .find(|l| l.name == "Headless loadout")
                .expect("created")
                .id
                .clone();
            g.command(
                Command::SetDefaultLoadout {
                    id: loadout.clone(),
                },
                cx,
            );
            assert!(
                g.loadouts
                    .loadouts
                    .iter()
                    .any(|l| l.id == loadout && l.is_default)
            );
            g.manager = None;
            g.loadout = Some(loadout.clone());
            g.reload(cx);
            assert_eq!(g.totals.base.0, 0);
            g.toggle_member(row.id.clone(), true, "base".into(), cx);
            let base = g.totals.base;
            assert_eq!(base, oz_to_g(2.) * 2);
            g.toggle_member(row.id.clone(), false, "base".into(), cx);
            assert_eq!(g.totals.base.0, 0);
            g.undo(false, cx);
            assert_eq!(g.totals.base, base);
            g.undo(true, cx);
            assert_eq!(g.totals.base.0, 0);
            g.toggle_member(row.id.clone(), true, "base".into(), cx);
            g.add_gear(w, cx);
            let e = g.editor.as_mut().expect("wish draft");
            e.status = "wishlist".into();
            type_text(&e.name, "Headless wish", w, cx);
            type_text(&e.weight, "1", w, cx);
            g.save(w, cx);
            let wish = g
                .all_gear
                .iter()
                .find(|r| r.name == "Headless wish")
                .expect("wish")
                .id
                .clone();
            let membership = g.membership(&loadout);
            let before = strider_core::gear::wishlist_projection(
                &g.core_rows(),
                Some(&membership),
                &BTreeMap::new(),
            );
            g.command(
                Command::SetWishlistReplacements {
                    item: wish.clone(),
                    replaces: vec![row.id.clone()],
                },
                cx,
            );
            let after = strider_core::gear::wishlist_projection(
                &g.core_rows(),
                Some(&membership),
                &BTreeMap::new(),
            );
            assert!(after.projected_base_g < before.projected_base_g);
            assert_eq!(after.projected_base_g, i64::from(oz_to_g(1.).0));
            g.undo(false, cx);
            assert!(
                g.all_gear
                    .iter()
                    .find(|r| r.id == wish)
                    .expect("wish")
                    .replaces
                    .is_empty()
            );
            g.undo(true, cx);
            assert_eq!(
                g.all_gear
                    .iter()
                    .find(|r| r.id == wish)
                    .expect("wish")
                    .replaces,
                vec![row.id]
            );
        })
    });
    cx.run_until_parked();
}
#[gpui::test]
fn food_volume_measure_reference_and_cross_area_undo_redo(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    cx.update(|w, cx| {
        view.update(cx, |g, cx| {
            let before = g.store.ingredients().expect("ingredients");
            g.area = "Food";
            g.open_ingredient(None, w, cx);
            let e = g.food.ingredient_editor.as_ref().expect("ingredient form");
            type_text(&e.name, "Headless sugar", w, cx);
            type_text(&e.kcal, "400", w, cx);
            type_text(&e.density, "0.8", w, cx);
            g.save_ingredient(w, cx);
            assert!(g.food.ingredient_editor.is_none());
            let ingredient = g
                .food
                .ingredients
                .iter()
                .find(|i| i.input.name == "Headless sugar")
                .expect("ingredient")
                .id
                .clone();
            g.create_meal(w, cx);
            let meal = g.food.editor.as_ref().expect("meal editor").id.clone();
            g.add_pantry_line(ingredient.clone(), cx);
            g.sync_meal_lines(w, cx);
            let line = g
                .food
                .meals
                .iter()
                .find(|m| m.id == meal)
                .expect("meal")
                .items[0]
                .id
                .clone();
            let draft = g
                .food
                .editor
                .as_mut()
                .expect("editor")
                .lines
                .get_mut(&line)
                .expect("line draft");
            draft.unit = "tbsp".into();
            type_text(&draft.amount, "2", w, cx);
            g.save_line(line.clone(), cx);
            assert!(g.error.is_none(), "{:?}", g.error);
            let meal_row = g
                .food
                .meals
                .iter()
                .find(|m| m.id == meal)
                .expect("reloaded meal");
            let grams =
                strider_core::food::grams_from_volume(2., "tbsp", Some(0.8)).expect("grams") as i32;
            assert_eq!(meal_row.items[0].amount_g, grams);
            assert_eq!(meal_row.items[0].entered_amount, Some(2.into()));
            assert_eq!(meal_row.items[0].entered_unit.as_deref(), Some("tbsp"));
            assert_eq!(meal_row.total_kcal, i64::from(grams) * 4);
            g.undo(false, cx);
            assert_eq!(
                g.food
                    .meals
                    .iter()
                    .find(|m| m.id == meal)
                    .expect("meal")
                    .items[0]
                    .amount_g,
                50
            );
            g.undo(true, cx);
            assert_eq!(
                g.food
                    .meals
                    .iter()
                    .find(|m| m.id == meal)
                    .expect("meal")
                    .items[0]
                    .amount_g,
                grams
            );
            // Density commands propagate into volume-entered recipe lines.
            g.command(
                Command::SetIngredientMeasure {
                    id: ingredient.clone(),
                    unit: "tbsp".into(),
                    grams: 10.into(),
                },
                cx,
            );
            assert_ne!(
                g.food
                    .meals
                    .iter()
                    .find(|m| m.id == meal)
                    .expect("meal")
                    .items[0]
                    .amount_g,
                grams
            );
            let sugar = strider_store::reference::search_reference_foods("sugar", 1, 0.5).remove(0);
            g.command(
                Command::LinkIngredientReference {
                    id: ingredient.clone(),
                    density: strider_core::format::fixed(sugar.density_g_ml.expect("density"), 6)
                        .parse()
                        .expect("decimal"),
                    reference_name: Some(sugar.reference_name),
                },
                cx,
            );
            assert_eq!(
                g.food
                    .ingredients
                    .iter()
                    .find(|i| i.id == ingredient)
                    .expect("linked")
                    .input
                    .density_source
                    .as_deref(),
                Some("usda")
            );
            g.area = "Gear";
            g.add_gear(w, cx);
            let e = g.editor.as_ref().expect("gear");
            type_text(&e.name, "Cross-area undo", w, cx);
            g.save(w, cx);
            g.undo(false, cx);
            assert!(!g.all_gear.iter().any(|r| r.name == "Cross-area undo"));
            g.undo(false, cx);
            assert_ne!(g.food.ingredients, before);
            g.undo(true, cx);
            assert_eq!(
                g.food
                    .ingredients
                    .iter()
                    .find(|i| i.id == ingredient)
                    .expect("redo link")
                    .input
                    .density_source
                    .as_deref(),
                Some("usda")
            );
            g.undo(true, cx);
            assert!(g.all_gear.iter().any(|r| r.name == "Cross-area undo"));
        })
    });
    cx.run_until_parked();
}

#[gpui::test]
fn meal_invalid_save_navigation_guard_and_unit_preservation(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    cx.update(|w, cx| {
        view.update(cx, |g, cx| {
            g.area = "Food";
            g.create_meal(w, cx);
            let id = g.food.editor.as_ref().expect("editor").id.clone();
            let ingredient = g
                .food
                .ingredients
                .iter()
                .find(|i| i.input.density_g_ml.is_some())
                .expect("density fixture")
                .id
                .clone();
            g.add_pantry_line(ingredient, cx);
            g.sync_meal_lines(w, cx);
            let before = g.food.meals.clone();
            let line = g
                .food
                .meals
                .iter()
                .find(|m| m.id == id)
                .expect("meal")
                .items[0]
                .id
                .clone();
            let e = g.food.editor.as_ref().expect("editor");
            type_text(&e.name, "Pending rename", w, cx);
            type_text(&e.lines[&line].amount, "invalid", w, cx);
            g.save_meal(cx);
            assert!(g.error.is_some());
            assert_eq!(g.food.meals, before, "validate all lines before any write");
            g.request_navigation(super::Navigation::Area("Gear"), w, cx);
            assert!(g.pending_navigation.is_some());
            assert_eq!(g.area, "Food");
            g.close_overlays(w, cx);
            assert!(g.food.editor.is_some(), "Escape retains meal edits");
            let d = g
                .food
                .editor
                .as_mut()
                .expect("editor")
                .lines
                .get_mut(&line)
                .expect("line");
            d.unit = "tbsp".into();
            type_text(&d.amount, "2", w, cx);
            g.save_meal(cx);
            assert!(g.error.is_none(), "{:?}", g.error);
            assert_eq!(
                g.food.meals.iter().find(|m| m.id == id).expect("meal").name,
                "Pending rename"
            );
            assert!(!g.meal_dirty(cx));
            g.toggle_weight_view(w, cx);
            let e = g.food.editor.as_ref().expect("editor");
            assert!(e.as_weight);
            type_text(&e.lines[&line].amount, "40", w, cx);
            g.save_line(line.clone(), cx);
            let saved = &g
                .food
                .meals
                .iter()
                .find(|m| m.id == id)
                .expect("meal")
                .items[0];
            assert_eq!(saved.amount_g, 40);
            assert_eq!(saved.entered_unit.as_deref(), Some("tbsp"));
            g.toggle_weight_view(w, cx);
            let e = g.food.editor.as_ref().expect("editor");
            type_text(&e.preview, "2", w, cx);
            g.set_preview(w, cx);
            let e = g.food.editor.as_ref().expect("editor");
            type_text(&e.lines[&line].amount, "4", w, cx);
            g.save_line(line.clone(), cx);
            let saved = &g
                .food
                .meals
                .iter()
                .find(|m| m.id == id)
                .expect("meal")
                .items[0];
            assert_eq!(saved.entered_amount, Some(2.into()));
            assert_eq!(saved.entered_unit.as_deref(), Some("tbsp"));
            g.request_navigation(super::Navigation::Area("Gear"), w, cx);
            assert_eq!(g.area, "Gear");
            assert!(g.pending_navigation.is_none());
        })
    });
    cx.run_until_parked();
}

#[gpui::test]
fn inventory_keeps_a_list_viewport_at_half_width(cx: &mut TestAppContext) {
    let (_, mut cx) = headless(cx);
    cx.simulate_resize(gpui::size(gpui::px(640.), gpui::px(480.)));
    cx.run_until_parked();
    let half = cx
        .debug_bounds("inventory-column")
        .expect("stacked inventory column");
    assert_eq!(half.size.height, gpui::px(500.));
    let body = cx.debug_bounds("inventory-body").expect("scrolling page");
    assert!(body.size.height > gpui::px(0.) && body.size.height < gpui::px(480.));
    cx.simulate_resize(gpui::size(gpui::px(1180.), gpui::px(820.)));
    cx.run_until_parked();
    let wide = cx
        .debug_bounds("inventory-column")
        .expect("desktop inventory column");
    assert!(wide.size.width > half.size.width);
    assert!(wide.size.height > gpui::px(0.));
}
