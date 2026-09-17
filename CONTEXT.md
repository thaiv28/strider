# Strider

Strider is a personal planning system for backpacking equipment, routes, food,
and completed trips.

## Language

**Gear Item**:
A physical item that is current, retired, or being considered on the wishlist.
_Avoid_: Product, asset

**Gear Kit**:
A Gear Item whose weight is derived from a fixed list of components.
_Avoid_: Bundle

**Loadout**:
A reusable selection of Gear Items with a base-or-worn classification.
_Avoid_: Pack, packing list

**Trip**:
A planned or completed backpacking outing with frozen gear and food snapshots.
_Avoid_: Hike, expedition

**Trail**:
A reusable destination or route that can be associated with multiple Trips.
_Avoid_: Trip, route plan

**Trip Gear**:
The immutable-at-completion snapshot of Gear Items selected for a Trip.
_Avoid_: Loadout

**Meal**:
A reusable recipe composed of Ingredients and measured in servings.
_Avoid_: Food item

**Ingredient**:
A reusable food component with nutrition measured per 100 grams.
_Avoid_: Meal

**Trip Meal**:
A frozen Meal or Ingredient allocation for one day of a Trip.
_Avoid_: Meal

**Base Weight**:
The weight of packed, non-consumable Trip Gear.

**Worn Weight**:
The weight of Trip Gear worn or carried outside the pack.

**Consumable Weight**:
The weight of food, water, and fuel expected to decrease during a Trip.

**Pack Weight**:
Base Weight plus Consumable Weight.

**Skin-out Weight**:
Pack Weight plus Worn Weight.
