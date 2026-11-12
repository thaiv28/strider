# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Primarily Thai, the owner and builder, planning personal backpacking trips.
Trip partners join through shared trip links. Sign-in is open to any verified
Google account, and each account gets a private workspace, but Strider is not
marketed to the public.

## Product Purpose

Strider is a personal planning system for backpacking: gear, routes, food and
completed trips, in one place from first idea to trailhead. Success means the
whole plan (route, load, food) is accurate and connected, so a change in one
part shows up in the others.

## Positioning

The trip plan stays connected. A trip's GPX route, daily mileage and gain,
campsites, gear snapshot, meal plan and carried weight all live together and
update each other. Neighboring tools cover only one of these (maps, gear lists,
or meal planners).

## Operating Context

- Trips move from idea to planned to completed. Gear and food snapshots freeze
  when a trip is completed.
- Trip partners open revocable view or edit links. Printable trip sheets go
  into the pack.
- Used on phones at 360–430 CSS px, half-screen and desktop. Light, dark and
  system appearance.

## Capabilities and Constraints

- Domain language is in `CONTEXT.md`: Trip, Trail, Gear Item, Gear Kit,
  Loadout, Meal, Ingredient, Trip Meal, Base Weight.
- Route map with topo/OSM layers, GPX upload, campsites, per-day distance and
  elevation, elevation profile.
- Gear inventory, kits, loadouts, and base/worn/consumable weight classes.
- Meals and ingredients with nutrition, trip food planning by day, and calorie
  estimates.
- Weather, snow depth, calendar feeds, sharing and printable sheets.
- Sign-in is Google only (Auth.js). A test credential form appears only with
  `?test=1` for automated tests.
- Weights are stored as grams and money as integer cents; they are converted
  only at the UI edge.

## Brand Commitments

- Name: Strider. The visual and interaction contract is `DESIGN.md` (Topo).
  Changing it needs Thai's explicit approval.
- Voice: practical, quiet and confident. Use few words.

## Evidence on Hand

- No testimonials, user counts, press or customer stories exist. Do not
  invent them.
- Product visuals should be faithful recreations of real Strider UI with
  plausible sample trip data, not screenshots. The real UI lives in
  `components/`.

## Product Principles

1. Show the product, don't describe it.
2. The route and the numbers are the content.
3. Fewer words. Every sentence earns its place.
4. The plan stays connected across route, gear and food.
