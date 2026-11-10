# Strider design: Topo

This is the visual and interaction contract for Strider. It records the Topo
design as shipped in #14, and every UI change must follow it. The palette and
primitive inventory lives in `docs/handoff/CONVENTIONS.md` ("UI palette"); this
file explains how to use them and what to leave out.

**Changing the design itself needs Thai's explicit approval.** Before adding a
new token, color, font, primitive, layout pattern, or navigation entry,
describe the proposal and wait for approval. Do not ship it and ask afterwards.
Approved changes update this file in the same PR.

## Character

Strider should feel like a calm field notebook laid over a topo map. Deep
forest-green header bands carry faint contour lines. Content sits on quiet
off-white panels, numbers are precise and tabular, and one warm route orange
marks the trail. The tone is practical, quiet and confident: the route and the
numbers are the content, and the interface stays in the background.

## Principles

1. **Content first, chrome last.** Each screen has one obvious subject (a trip's
   route, the gear list, the meal plan), and everything else supports it.
2. **Restraint.** Prefer fewer elements, fewer colors and fewer words. If a
   label, icon, badge, border or helper sentence can be removed without losing
   meaning, remove it.
3. **Extend before adding.** A new capability should first live in an existing
   surface: a row, a menu item, a field in an existing card, a section of an
   existing sheet. Add a new card, tab, page or nav item only when it cannot.
4. **Progressive disclosure.** Put secondary detail behind an `InfoBadge`, a
   `Sheet`, a menu (as route actions are grouped in one menu), or a collapsed
   state. Show it inline only when people need it in the moment.
5. **One system.** Use the primitives in `components/ui.tsx` and the
   layout components, never a lookalike. If a primitive almost fits, propose
   a variant rather than restyling it locally.
6. **Every width matters.** Design for phone (<640px), half-screen
   (640–1099px) and desktop (`desk:` 1100px+) together.

## Foundations

### Color roles

Use only the semantic tokens from `app/globals.css`, through Tailwind
utilities (`bg-panel`, `text-muted`, `border-fieldborder`). Never use hex,
`rgb()`, Tailwind palette colors (`text-red-600`, `bg-white`) or
`bg-[var(--x)]`. Opacity modifiers on tokens (`bg-accent/10`) are fine.

| Role | Tokens | Use for |
| --- | --- | --- |
| Surfaces | `canvas`, `panel`, `panel2` | Page background, cards and sheets, inset/hover/secondary fills |
| Text | `ink`, `muted` | Primary text; secondary text, labels and metadata |
| Lines | `line`, `divider`, `fieldborder` | Row separators and card edges; section dividers; form control borders |
| Primary | `accent`, `accentink` | Primary buttons, selected chips/tabs/segments, focus rings, links on hover |
| Warm emphasis | `signal` | Sparing: inline text actions ("All trips", "Add day"), "Next up", conflicts, savings, destructive actions |
| Route | `route` | The route line and route-specific highlights only |
| Weight classes | `cbase`, `ccons`, `cworn` | Base, consumable and worn weight only, via `Badge` tones and weight bars |
| Header band | `header*` tokens | Inside `PageHeader`, the app shell header and the phone trip header only |
| Map | `map`, `contour`, `lake`, `elevation` | Map placeholders, contour art, the elevation profile |

Category colors come only from `categoryColor()` in `lib/categories.ts`.
Dark mode is the neutral charcoal set in `globals.css`. Any new token needs a
light and a dark value, and it is a design change.

### Typography

Two families, both self-hosted: **Instrument Sans** (`font-sans`, the default
for everything) and **Bricolage Grotesque** (`font-display`, titles and big
numbers only). No other fonts or `font-[…]` values.

| Use | Style |
| --- | --- |
| Page title (`PageHeader`) | `font-display text-[32px] sm:text-[44px] font-bold` |
| Sheet title, featured card title | `font-display text-[26px] font-bold` |
| Section heading on a page | `font-display text-[22px] font-bold` |
| Card heading | `text-[17px] font-bold` (sans) |
| Body, controls | `text-sm` |
| Field label | `text-[13px] font-semibold text-muted` (use `Field`) |
| Small label | `eyebrow` class |
| Metadata | `text-xs text-muted` |
| Numbers (weights, miles, cents, counts) | `readout` class or `tabular-nums` |

Write in sentence case. Use the units people backpack with ("oz", "lb", "mi",
"ft") and convert from grams or cents only at the UI edge.

### Shape, depth and spacing

- Cards use the `card` class or `Card`: a `panel` fill, no visible border and
  14px radius. Don't put cards inside cards; inside a card, use `border-b`
  rows or `panel2` fills.
- Controls are `rounded-xl` (buttons, chips, tabs, icon buttons) or
  `rounded-[10px]` (inputs, selects), and pills are `rounded-full`. Sheets
  are 20px, or 22px at the top on phones.
- Depth is flat. Shadows are reserved for things that float: the route card,
  menus and popovers, the undo toast, and controls over the map.
- Interactive targets are at least 44px high (`min-h-11`, `h-11 w-11`).
- Typical card padding is `p-4` (`p-5` for a featured card). Stacks use
  `space-y-5`/`space-y-6` between sections and `gap-3`/`gap-3.5` in grids.
  Page content is capped at `max-w-[1136px]` through `PageBody`.

### Icons and motion

- Use `lucide-react` icons at 16–21px for controls and 23px in the phone
  bottom nav. An icon needs a job: a control or a nav target, not decoration
  next to a heading. Icon-only buttons use `IconButton` with an `aria-label`.
- Motion is limited to short color and opacity transitions (`transition`).
  Decorative motion is only for the landing page, and every animation respects
  `prefers-reduced-motion`.

## Layout and components

- **App shell** (`components/shell.tsx`): a forest header band with contours.
  Primary navigation is Basecamp, Gear, Food, Trips, and Calendar. On phones
  it is a bottom nav; from `sm:` it moves into header links, plus Settings. The
  nav is complete: new features live inside these areas.
- **Page** = `PageHeader` (title, optional actions, optional summary stats in
  the band) + `PageBody`, which overlaps the band. Don't build other page
  headers.
- **Trip page** is map-first. The route card (map and elevation) stays visible
  across trip sections and can be minimized; desktop uses a split view and
  phones use a trip section bar. Sections switch with `Chip`s, and route actions
  live in one menu. New trip features become content within a section, not new
  persistent chrome.
- **Overlays**: `Sheet` for every dialog. It is a bottom sheet on phones and a
  centered dialog elsewhere. Don't create new modal styles.
- **Choosing a control**: use `PageTabs` to switch views of a page,
  `SegmentedControl` for a mode or unit choice, `Chip`s for filters and
  sections, `Select` for longer option lists, and `WornToggle` for pack/worn.
- **Buttons**: one `primary` action per region at most. Secondary actions use
  `outline` or `ghost`, inline text actions use `font-semibold text-signal`,
  and destructive actions use `danger`.
- **Badges**: `Badge` with an existing tone. Show a status badge only when the
  status is decision-relevant on that screen.
- **Empty states**: one short muted sentence that says what to do next
  ("No days yet. Add a day, or drop campsites on the route."). No illustrations,
  icons or extra buttons beyond the one action.
- **Forms**: `Field` for label and hint. Keep hints short or move them into an
  `InfoBadge`. Forms save in place where the surrounding screen does.

## Clutter budget for new features

Before adding UI, check:

- [ ] Could this be a row, a menu item, or a field in something that already exists?
- [ ] Does it add a new card, tab, page, nav item, toolbar, banner or persistent button? If so, justify it or redesign.
- [ ] Is each new word, icon, badge and border necessary?
- [ ] Is secondary detail disclosed on demand rather than always shown?
- [ ] Does it use only existing primitives, tokens and type styles?
- [ ] Does it work and look intentional at phone, half-screen and desktop widths, in light and dark?

If a feature needs something the system lacks, stop and propose the addition
before writing it.

## Reference screens

These are the quality bar; new screens should feel like they belong with them:

- **Basecamp** (`components/home/home-view.tsx`): a header band with
  stats, a featured "Next up" card, and a compact recent-trip grid.
- **Trip page** (`components/trip/trip-view.tsx`): a map-first split,
  section chips, quiet detail cards.
- **Gear** (`components/gear/`): dense, scannable rows with tabular weights
  and weight-class color used only for meaning.

## Enforcement

`npm run check:design` (`scripts/check-design.mjs`) runs in CI and before
deploy. It fails on raw colors, Tailwind palette colors, `bg-[var(--x)]`
arbitrary values, ad-hoc fonts or font imports, and new stylesheets outside
`app/globals.css`. Existing, deliberate exceptions are listed in its
`BASELINE`, with a reason, as per-file maximum counts. Those exceptions are
map legends matched to NOAA tiles, map-overlay controls on light tiles,
generated images, and Google's sign-in button. Raising a count or adding a
file there is a design change and needs approval.

The script cannot judge composition or clutter. Those are reviewed against this
file.
