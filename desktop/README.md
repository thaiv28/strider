# Strider Desktop — milestone 5a (native chrome and PDF trip sheet)

Native Rust/GPUI Gear and Food areas on a local SQLite workspace.
Gear includes inventory, kits, loadouts, category management and wishlist projections.
Food includes Meals, Ingredients, measured recipe lines and bundled USDA search.
Device sign-in and background push/pull synchronization use the versioned `/api/v1`
server API. Settings contains Account controls. Trips, Basecamp and the Trip page are
implemented (below), as are the offline map download, sync status with conflicts,
Shared with me, the share menu, the weather and snow panels, Calendar and the full
Settings area; packaged-product lookup is not implemented.

Run from `desktop/` so rustup selects the pinned toolchain:

```sh
cargo run -p strider-app --release
cargo run -p strider-app --release -- --reset
# Everything needed is cached; offline development:
cargo run --offline -p strider-app -- --reset
cargo run --offline -p strider-app -- --profile demo --reset
```

The database is `$HOME/Library/Application Support/Strider/<profile>/workspace.db`;
`--profile <name>` defaults to `dev` and accepts letters, digits, `-`, and `_`.
The app creates its parent directories, enables foreign keys and WAL, and applies
numbered migrations using `PRAGMA user_version`. Newer schemas are refused.
The old milestone-0 schema is explicitly refused; use `--reset` for that
disposable fixture. `--reset` deletes this workspace file
and starts an empty workspace. New profiles are empty; fixtures are test-only. Fixture values and valid UUIDv7 identities are deterministic;
the identities use a fixed fixture timestamp and monotonic payloads. Undo history
lives only in memory and disappears at quit or after a server pull.

The pinned pair is `gpui-pre`/`gpui-pre-platform` **0.3.12**, used with
`gpui-component` **0.9.1**. Upgrade them together. The platform's `runtime_shaders`
feature compiles Metal shaders at launch, allowing builds with Apple's Command
Line Tools instead of requiring the full Xcode shader compiler. `font-kit`
supports font discovery; the UI embeds Instrument Sans and Bricolage Grotesque.
Startup uses `gpui_platform::application()` and initializes gpui-component before
wrapping the application view in its `Root`.

| Crate | Responsibility |
| --- | --- |
| `strider-core` | Units, decimal arithmetic, formatting, food, fuel, energy, route, gear, trip, GPX and propagation rules; regex/serde for GPX |
| `strider-store` | Local workspace schema, deterministic fixture, Gear/Food/Trip queries and commands, outbox batches, undo/redo |
| `strider-sync` | Blocking HTTP, loopback PKCE sign-in, Keychain credentials, durable push/pull and background scheduling |
| `strider-ui` | Generated Topo colors, embedded fonts, primitives, icons and contour drawing |
| `strider-app` (`strider`) | Windows and shell, menu bar and key map, command palette, Gear inventory/wishlist, Food pantry/meal editors, Trips, actions and drag/drop |
| `strider-print` | Letter PDF trip sheet: layout, embedded fonts, cached-tile map raster |
| `strider-devices` | Mounted GPS volumes (send a GPX), the watched folder, FIT import and per-device preferences |

`strider-ui/build.rs` reads the Topo light/dark blocks in `app/globals.css` and
fails on property mismatches. Category colors are the ported web table. The
Rust design guard rejects raw colors and literal font families outside the UI
implementation. No web stylesheet is changed.

## Interaction

Click a row to select it; double-click or use its more button to edit. Expand a
kit with its chevron. Search by item or category name; More exposes Current /
Retired / All, bulk loadout membership and Manage loadouts.
Wishlist items never enter the inventory. Drag a grip to insert before another
row; drop on a category header to append to it. Dragging is disabled while searching.
Select a loadout above BaseBar to check membership and mark members packed/worn.
The list and BaseBar use `strider-core::gear` subtotals; BaseBar stacks above the
list below 1100px. Hover or focus its segments or info controls to inspect items.
Wishlist has its own search and loadout selection, replacement checkboxes, explicit
include/exclude overrides and automatic selection reset. Its projection uses the
same core rules, including deduplicated replacements.

Add gear opens the full Sheet: category, oz, quantity, USD, class, status, kit
components, URL and notes. Kit weight is computed from its editable components.
Delete errors remain inline in the Sheet. Loadouts and categories use inline
name inputs and ↑/↓ ordering controls.

Food opens on Meals. Select a meal for the editor beside the list at 1100px+;
smaller windows show the editor alone. Save persists recipe text, base servings,
water and all valid amount edits. Type and hot/cold choices persist immediately.
Lines accept g/oz or tsp/tbsp/cup/fl oz/mL/L and keep their entered amount/unit.
Set commits a line; View as weight preserves the original entered unit. Show
changes the serving preview. Lines can be reordered or removed. Pantry matches
precede USDA results when adding a line. Ingredients supports manual entry,
reference-food creation, density/source editing, measured volume weights and
linking a saved ingredient to a suggested USDA density. Invalid volume entry
without density gives an inline error; Measure / link opens that ingredient.
Unsaved meal text/amount edits have Save and leave / Discard / Keep editing;
untouched new meals are discarded when leaving.

All user writes call `Store::apply(Command)` and reload SQLite query models for
the returned `ChangeSet` (currently conservatively reloading all Gear/Food models).
Only field edits and view choices are kept locally. Undo/redo re-reads both areas
and closes the meal editor so it cannot retain stale fields.

### Keyboard map

Every command is one GPUI action, bound below and reachable from the menu bar; menu items
disable themselves when the focused window cannot handle the action (for example Export GPX
with no routed Trip open).

| Shortcut | Action | Menu |
| --- | --- | --- |
| ⌘, | Settings | Strider |
| ⌘H / ⌥⌘H | Hide Strider / Hide Others | Strider |
| ⌘Q | Quit | Strider |
| ⇧⌘N | New Window | File |
| ⌘N | New Gear (in Gear), New Trip (in Trips and Basecamp) | File |
| ⌘O | Import GPX… (a GPX, permit or Trip JSON into the open Trip) | File |
| ⌘E | Export GPX… | File |
| ⌘P | Print Trip Sheet… | File |
| ⌘W | Close Window | File |
| ⌘Z / ⇧⌘Z | Undo / Redo workspace command (native text undo inside inputs) | Edit |
| ⌘X ⌘C ⌘V ⌘A | Cut, Copy, Paste, Select All | Edit |
| ⌘F | Focus the current Gear/Wishlist or Food search | Edit |
| ↑ / ↓, Return, ⌫ | Select previous/next item, edit it, delete it (Gear) | Edit |
| ⌘K | Command palette | View |
| ⌘1 ⌘2 ⌘3 ⌘4 ⌘5 | Gear, Food, Trips, Calendar, Basecamp | View |
| ⌃⌘F | Enter Full Screen | View |
| ⌘R | Sync Now | Sync |
| ⌘M | Minimize | Window |
| Escape | Close the palette, a menu or a Sheet; leave search focus; guarded meal edits are kept | |

View → Appearance has Light / Dark / System. Sync also has Show Sync Status and Sign In… / Sign
Out (only the one that applies is enabled). Window lists the open windows (macOS adds them);
Help → Strider on the Web opens https://strider.thaiv.dev. ⌘N is bound per key context
(`in-gear`, `in-trips`, `in-basecamp`), so the same chord does the right thing per area.

### Command palette

⌘K opens a Sheet listing every command that currently applies plus a "Go to" entry for every
Trip, gear item and meal. Typing fuzzy-matches (case-insensitive subsequence; consecutive
runs, word starts and prefixes score higher, longer labels cost a little). ↑/↓ choose,
Return runs, Esc closes. At most 12 results show at once; keep typing to narrow them.

### Multiple windows

New Window (⇧⌘N) or a Trip row's right-click menu, "Open in New Window", opens another `AppView`
over the **same** `Store` handle (`Rc<RefCell<Store>>`; every borrow is short and never held
across a callback). Any command, undo/redo or sync pull in one window reloads every other
window, and undo/redo stay one global history. Each window keeps its own area, selection and
open Sheet. A window's title is its area, or the open Trip's name.

The first window owns sign-in and background sync. Sync menu items and the Settings account
card in other windows forward to it (and bring it forward); if it closes, the account and its
sync thread pass to the oldest remaining window. Closing the last window quits the app.

The window frame (position, size, maximized/full screen) and the area are saved after each
change into the workspace's local, unsynced `sync_state` table (`window_frame`, `window_area`)
and restored at the next launch; a frame that is off every connected display is ignored.
Only the first window is restored; secondary windows are not reopened.

### PDF trip sheet

File → Print Trip Sheet… (⌘P) opens a save panel named `<gpx_filename stem>.pdf` (the same
stem as Export GPX), renders the open Trip off the UI thread, writes the file and opens it with
`open`. `crates/strider-print` builds the document with `pdf-writer`: Letter pages, text in the
bundled Bricolage Grotesque and Instrument Sans (embedded, with a ToUnicode map, so text is real
and selectable), colors from the light Topo tokens. Order: title, dates and nights, place,
distance/gain/trailhead; the per-day Distance / Gain / Camp table with totals; Permit notes and
the permit document reference; Gear (weight by class, then by category); Food per day;
Campsites; then a full-page route map. The map is rasterized with `tiny-skia` from tiles
already in the disk cache (`strider_map::read_cached`; the sheet never fetches), with parent
tiles standing in for missing ones, the route drawn over them and the numbered campsites drawn
as vector markers with real text. With no cached tiles the route is drawn over the `map` token
color alone. Output is deterministic for a fixed input (no dates or random ids; compressed
streams use a fixed zlib level).

Differences from the web print page: the web page has no Gear, Food or Campsites sections,
shows dates as "Jun 3, 2026 – Jun 6, 2026", prints an image permit on its own page, and prints
from a browser; the desktop sheet adds those sections, uses the app's compact date range, and
references an image permit by filename like a PDF one.

Text inputs retain their native text editing shortcuts. Inventory
selection/deletion is suspended while the Sheet is open. Weight accepts decimal
oz, converts to integer grams on save, and allows blank for unknown weight.
Quantity must be a positive integer. Invalid numeric input displays
“Enter a number.”; overflow and database errors are surfaced without applying
partial edits. The Sheet closes with Cancel, its close button, scrim, or Escape.

## Verification

```sh
cargo build --offline --workspace
cargo test --offline --workspace
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo fmt --check
```

Tests include domain golden fixtures, USDA top-three search goldens derived from
the unchanged TypeScript over the bundled index, decimal rounding and propagation,
SQLite schema/command invariants, the Rust design guard, and GPUI headless workflows.
The workflows create gear through the dialog, edit kit contents, check loadout
membership/totals/defaults, project wishlist replacements, create meals, enter
pantry volumes, propagate measured/reference densities, and undo/redo across areas.
They also cover invalid meal saves, unsaved navigation, serving previews and
preserving entered units in weight view, plus a 640px/1180px retained-list layout
check. The original Gear view test remains: The headless test loads 2,000
items, dispatches SelectNext twice, deletes the selected item, and verifies Undo
restores the complete grouped result including component identities. It also
checks invalid numeric edits keep the Sheet open and Escape closes it from a
focused input.

## Frame logging

```sh
STRIDER_FRAME_LOG=1 cargo run --offline -p strider-app --release
```

At entry to each Gear root-view `render`, record an `Instant` and the interval
since its preceding render. Once at least a second has elapsed and a new render
arrives, sort that interval batch and log nearest-rank p95, maximum, and sample
count to stderr, then clear the batch. Logging does not schedule extra frames.
Idle intervals are included; a report after a pause can contain that pause.
These are root-render cadence measurements, not CPU render duration, GPU timing,
or presentation latency. Scrolling that only redraws GPUI's retained list may
not rerender the Gear root and therefore may not add a sample. Compare release
builds under the same interaction sequence; this metric alone cannot establish
smoothness.

## Spike notes

### Native primitives and input styling

GPUI does not consume CSS. Topo layout, type sizes, radii and semantics are
ported to reusable Rust builders; colors remain generated from the web tokens.
The component input has its own theme and background behavior, so initialization
also maps its font, border, caret, focus and selection colors, and inputs receive
an explicit panel background, 10px radius and 44px height. `FontFeatures`
supports `tnum`, so tabular readouts require no fallback. There is no toolkit
requirement to change the Topo palette, fonts or radii.

### Virtualized card slices

The permitted GPUI `list` handles variable-height flattened entries rather than
mounting entire category cards. Headers, rows, expanded components and 12px gaps
paint individual card slices; header and final-entry corners supply continuity.
Only visible entries render. The flat cache changes on data/filter/expansion,
with logical scroll position retained across commands and kit expansion.

### Sheet and icon integration

The Topo centered Sheet uses an occluding GPUI surface with app-owned focus,
scrim and Escape handling; it does not use the component library's differently
styled Sheet. Its height is capped with scrolling for the minimum window size.
The five icons are simple local paths, approximating the web's Lucide strokes,
as requested; there is no added icon crate. The shell uses tighter existing
spacing at desktop widths to retain all five links at the 640px minimum.

### Synchronous storage and undo memory

One UI `rusqlite::Connection` belongs to `Store`; queries and commands run
synchronously on the main thread, acceptable for this spike. Each reload fetches
items/components in bulk. A command stores complete before/after snapshots as
its inverse, so delete/undo restores exact component ids and ordering, including
hidden Wishlist rows. This is deliberately simple but memory grows with edit
history; it is not a production undo architecture.

### Measurement and visual verification limits

Root-render intervals have the limits described above. Automated headless tests
exercise action routing and state restoration, not pixel parity, physical drag
feel, macOS accessibility behavior, or sustained 2,000-row scroll performance.
Those require a visual and interaction comparison with the web app on a Mac.

Milestone 1b validation uses GPUI headless tests only. The GUI is deliberately
not launched in this resource-constrained session; pixel parity, physical drag
feel and native accessibility still need host-side visual verification.

Regenerate domain golden fixtures from the repository root with `npx tsx scripts/export-domain-fixtures.ts`.
If a sandbox denies the tsx CLI's IPC socket, `node --import tsx scripts/export-domain-fixtures.ts`
runs the same exporter without that socket. The exporter never queries PostgreSQL.

### Milestone 1b differences from the web UI

- Native GPUI dropdown menus, text controls, focus outlines and centered Sheets
  approximate the web primitives; no CSS, tokens, colors, fonts or radii were added.
  Gear retains its virtualized card slices. Text keyboard shortcuts remain native.
- **Category management is hidden.** The category sheet exists in code but has no entry
  point, by Thai's decision (2026-11-16), until the web has category management.
  Loadout names use inline fields rather than browser prompts, and their
  ordering uses ↑/↓. Meal lines also offer ↑/↓ ordering.
- BaseBar implements the requested category bar with hover/focus item popovers;
  the web's optional pie chart and category-detail modal are not ported. At half
  width the stacked page scrolls around a finite 500px virtual-list viewport.
- Meal amount **Set** and serving-preview **Show** are explicit commit controls;
  the web updates these fields continuously. Type and hot/cold changes persist
  immediately; recipe text and amounts use Save. Unsaved navigation uses a Sheet.
  Measurement/linking opens the ingredient Sheet rather than expanding below a line.
  Pantry deletion is immediate and undoable rather than using a browser confirmation.
- Packaged search is disabled with “Packaged search needs a connection.” Search
  uses bundled reference data, not remote density lookup. Ranking's final
  description tie-break uses lowercase Rust lexical order, not ICU English collation;
  rare punctuation/accent ties can differ. Seven top-three TypeScript goldens pass.
- There are no browser deep links to meals or trip back-links in this milestone.
  UI mutations are local and synchronous; a background thread synchronizes them.

### Module map

`strider-app/src/main.rs` owns startup/profile/reset flags; `app.rs` owns the
window, root, actions, routing and appearance; `shell.rs` owns header/nav.
`gear/` contains inventory, the virtual list, dialog, BaseBar, loadouts/categories
and wishlist. `food/` contains pantry, meals, meal editor, ingredient dialog and
reference search. `strider-ui/src/controls.rs` supplies the ported Select,
Textarea, SegmentedControl, WornToggle, InfoBadge, Badge tones and accent Checkbox.
`strider-store::reference` embeds `lib/usda-reference-data.json` using `include_str!`
and parses it once with `serde_json`; no new crates or online fetching are required.

## Port boundaries

Category create/rename/reorder/delete and meal-line reorder are local commands;
the web app has no corresponding category actions. Deleting a category makes its
items uncategorized and detaches its child categories, explicitly within the
command, while retaining the server's no-action foreign keys. Deleting gear
referenced by a trip is refused, as on the server.

Identities and foreign keys use UUIDv7 strings instead of server serial ids;
the synthetic Other category uses `None` instead of `-1`. Singleton indexes
replace per-user singleton constraints in this single-user workspace. Permits
store blob hashes instead of bytes, and trip access is a local checked column.
Device sign-in is implemented; sharing-token mutation and remote USDA density
lookup are not implemented. Ingredient creation uses the supplied density; reference
linking accepts already-resolved reference data so it works offline.

SQL propagation uses exact decimals and half-away rounding. Web display and
form calculations retain JavaScript floating-point rounding. `Dec` accepts
at most nine fractional places (additional trailing zeros are allowed), and
commands reject values exceeding server integer storage ranges. Invalid civil
dates return `None`; the CSS-only `cn` utility has no core equivalent. Shopping
names use lowercase lexical sorting instead of ICU `localeCompare`, since no
locale crate is available; accented and punctuation-heavy names can sort
differently. Other name sorting uses Rust lexical order instead of PostgreSQL's
configured collation; equal sort positions use UUID order for determinism.

Undo/redo restores exact stored before/after images, including propagated
snapshots and cascades, and emits a fresh outbox batch for the restoration.
Ordinary commands emit only their direct changes, excluding propagation.


## Device sign-in and synchronization

Use a fresh profile for sync; earlier milestones' deterministic fixture databases
are not a server import format. Existing fixture rows have no remote base state.
The profile binds to a server origin and email before its first push; switching
account or origin requires another profile. Sign-out retains local rows and this
binding. Tokens are generic passwords in macOS Keychain (`dev.thaiv.strider`,
account = server host), never SQLite values or logs. Keychain credentials are
shared by profiles for that host; use distinct hosts for distinct server accounts.

```sh
cargo run --offline -p strider-app -- --profile local-sync --server http://localhost:3100
```

Settings → Account → Sign in opens the browser using macOS `open`. The app uses
PKCE S256, a random state, and an ephemeral `127.0.0.1` callback with a five-minute
accept deadline. Sign-out waits for the worker's current sync pass before revoking
the device token. Failed revocation retains the credential and resumes the worker.

The worker owns a secondary WAL connection. It synchronizes at startup, after a
500 ms local-change debounce, on Sync now, and every minute. Transient failures
back off from two seconds to five minutes; Sync now explicitly retries immediately.
401 and 426 stop the worker and surface sign-in/update instructions.
Only offline status and waiting changes appear in the header. Its Sync Sheet shows
pending writes, elapsed time since last success, replaced text with Copy, and
rejections. Account errors also appear in Settings.

Each request contains at most 500 operations. Oversized local command batches are
split into persisted batch identities, so they lose atomicity across those chunks.
The exact in-flight JSON is saved before transmission and cleared with its ack;
a lost response retries the same payload and batch id after restart. Adoption
rewrites identities and foreign keys atomically. Rejected batches are logged,
local-only inserts without newer corrections are removed, and the pull cursor rewinds to refetch server
values. Pull preserves pending columns, updates long-text bases, applies revocation
cascades, and never appends undo or outbox entries. Because undo contains whole
workspace snapshots, any pulled row invalidates all undo/redo history, including
history held by a different UI connection.

Parents can appear on later pages because the API orders current row versions by
sequence. Such pages and their cursors are buffered atomically in SQLite until
all referenced parents are available, including across restart. A remaining
unresolvable dependency surfaces an error rather than dropping references.

Shared Trip library references must arrive as null when the owner's library is
not visible. Trip commands and queries work from snapshot columns in that case;
library propagation naturally skips null references. A server returning non-null
inaccessible parents still buffers the page and reports unavailable parents.
Shared snapshots cannot reconstruct recipe lines, meal hot/water settings, or a
trail name that the API does not expose: groceries fall back to snapshot names
and weights, and unavailable meal details contribute no cook water (as on web).

### Real local server tests (maintainer)

These tests are ignored by default; no app or server is launched by the tests.
Prepare an isolated local PostgreSQL-backed server and export
`STRIDER_TEST_SERVER`, `STRIDER_TEST_TOKEN_A`, and `STRIDER_TEST_TOKEN_B`.
A/B must be separate device sessions of the **same user**. Also export
`STRIDER_TEST_TOKEN_OTHER` from a **different user** to run the isolation test.
Keep tokens out of shell history. From `desktop/`:

```sh
cargo test --offline -p strider-sync --test server -- --ignored --test-threads=1
```

The four ignored tests cover seeded 200-step Gear/Food interleavings and full
row comparison across two devices and a fresh server pull; a replaced long-text
conflict; different-user isolation; and a two-device permit upload/download/delete round trip. A cleanup guard deletes uniquely named
`STRIDER-DESKTOP-TEST-*` records even on assertion failure. Without the third token,
run the two same-user tests individually with filters
`seeded_two_device_convergence_200_steps` and `replaced_long_text_is_saved`.
Local unit tests cover PKCE, callback validation, credentials, outbox conversion,
long-text bases, bounded batches, parent ordering, page buffering/restart,
adoption, revocation, pending columns, resurrection, atomicity and secondary
connection undo invalidation. Real browser sign-in, Keychain prompts, visual
layout and real-server convergence require maintainer verification; the GUI and
network-dependent integration tests are not run in the constrained sandbox.


## Milestone 3a: Trip commands, queries and permit bytes

No Trips UI is added. `strider-store/src/trips.rs` implements the Trip mutations;
`Command::CreateTrip` accepts optional local Trail and Loadout identities.
`UpdateTrip` patches and restore snapshots use checked snake_case SQLite column
names in `Row`; hidden fields and caller-supplied parent identities are refused.
File commands accept bytes and filenames. `AddTripFood` accepts a local recipe,
a pantry ingredient, or an already-resolved search hit (boxed `IngredientInput`).
Every command uses the existing transaction, ChangeSet and undo/redo path.
Trip day totals and campsite-generated days are computed after direct changes
and excluded from the ordinary outbox. Trip-command undo excludes those derived
changes too; existing library-command history retains its established behavior.
Manual edits to generated days wait for a pull to map the server's day identities
before their patches are pushed.

`trips_list()` uses the local civil date; `trips_list_on(date)` allows deterministic
upcoming/in-progress/past grouping, with completed Trips always past and undated
Trips upcoming. Within groups, dates descend, null dates last, UUID ties stable.
The current web logbook instead exposes status filters and a next-up card; the
three time groups are the explicitly requested desktop query contract.
`trip_detail(sync_id)` returns raw fields plus Trail, effective route stats,
days/segments, campsites, GPX, grouped snapshot gear/weights, food, groceries,
cook water, permit state and access. It hides private notes for collaborators.
Queries never append outbox writes or undo entries.

`strider-core::gpx` preserves the regex parser, hysteresis, 800-point downsampling
rule, JavaScript rounding, waypoint grouping and GPX handoff formatting. All 69
GPX fixture cases compare numbers exactly and export strings byte for byte.

Permit metadata contains no bytes. Files are hash-verified and atomically renamed
to `<data_dir>/blobs/<sha256>`, beside `workspace.db`; `blob_ref` tracks
`local_only`, `uploaded`, or `missing`. Upload intent is per Trip because several
Trips may attach the same content. The binary endpoint owns metadata creation;
only deletions use row sync. Uploads follow parent push/pull, downloads verify
both hash and size, and a pull adopts the server's permit identity. A deletion
made before that identity is known is retained until it can be sent. Files are
retained for undo; garbage collection is outside this milestone. In-memory test
stores use isolated temporary blob directories. Pending counts include uploads.

Excluded actions: create/revoke view links, create/revoke edit links, AI planning,
and `routeMapUrl`, as requested. JSON Trip import is supported, including fenced
JSON, inline tracks, campsite snapping and researched-gain fallback. It never
fetches `route.gpxUrl`: supply inline geometry or attach GPX bytes. Search hits
use supplied density instead of remote density lookup. Non-finite file-derived
storage values are rejected. Existing UUID tie-breaking and lexical shopping
ordering differences remain. Real-server convergence (four ignored tests) still
requires maintainer verification; offline fakes exercise the transfer boundary.

## Milestone 3b: Trips, Basecamp and the Trip page

Areas: **Trips** (grouped list with route thumbnails, New trip, Import trip JSON,
delete with undo), **Basecamp** (next trip, recent trips including completed, quick
links; no weather), and the **Trip page** (route card with a native raster
`RouteMap` and `ElevationProfile`; Logistics, Planning, Gear, Food and Report
sections; permit document; options; read-only for `view` access).

Keyboard: ⌘N New trip (Trips) or Add gear (Gear); ⌘O import/attach a file on a
Trip page (GPX, permit PDF/image, trip JSON); ⌘E export GPX; ⌘Z / ⇧⌘Z undo/redo;
⌘+ / ⌘− zoom the map (map focused). Dropping a `.gpx`, PDF or image on the window
attaches it to the open trip (a `.fit` works like a `.gpx`, see Devices). Map: drag or trackpad-scroll to pan, pinch or
double-click to zoom, click near the route (or the profile) to add a campsite,
drag a campsite marker to move it.

Tile cache: `~/Library/Caches/Strider/tiles/<source>/<z>/<x>/<y>` (falls back to
`<data dir>/cache` without `$HOME`), capped at 500 MB with least-recently-used
eviction, tiles kept at most 30 days, up to 4 concurrent fetches with
`User-Agent: StriderDesktop/<version>`. Sources match the web (USGS Topo to zoom 16,
OpenStreetMap to zoom 19). Without a cached or fetched tile the map shows the
`map` token background with the route and markers. Tests inject a fake fetcher
and never use the network.

Differences from the web (spike): rasters are drawn by GPUI, not MapLibre
(no vector styling, hillshade, or satellite); the list and trip details use
plain stacked cards and dialogs rather than the web's inline editors; gear/meal
reordering is by "Move up"/category choice rather than drag; permits open in the
system viewer from a temporary copy; weather, snow, share, print and Plan with AI
are absent; the elevation profile paints one stroked line over a filled area with
campsite lines instead of the web's per-day coloring.
The live Trip convergence test is `trip_two_device_convergence` in
`crates/strider-sync/tests/server.rs` (ignored; same environment variables).

## Milestone 4a: offline and sharing services (no UI yet)

Non-UI services for the Calendar, weather and snow panels, the route map and sharing. Nothing
here makes a network call in tests: public sources sit behind `strider_services::HttpClient`, the
signed-in server behind `strider_sync::Transport`, and recorded response bodies live in
`fixtures/http/`.

- **`strider-services`**: `weather` (`describe`, `is_far_future`, `fetch_day`,
  `fetch_climatology`, `add_days`, `pretty_date`, `clock`), `forecast_window`, `snow`,
  `calendar` (ICS parse and expansion, `calendar_data` month grid, `load_month`), `overpass`
  and `share` (`change_share_link`, `geocode`, `food_search`). Cached reads return
  `Cached { value, fetched_at, stale }`; `stale` means a refresh was needed but impossible, so
  the UI can say "Updated 3 h ago" offline. Freshness: forecast 1 h, archive 7 days,
  climatology 30 days, snow now 30 min, calendar feeds 5 min, Overpass 7 days.
- **Local caches** (migration 4, not synced, never in the outbox or undo): `weather_cache`
  (also holds snow), `calendar_cache` (raw ICS per feed; dropped when the feed is edited or
  deleted), `overpass_cache` (per Trip bbox) and `map_region`.
- **Offline map regions** (`strider_map::region`): a Trip's route bbox padded 10%, zooms 8..=15,
  USGS Topo only, refused above 20,000 tiles, at most 4 concurrent requests, resumable (tiles
  already on disk are pinned, not refetched), progress callback and cancel flag. Pinned tiles
  carry a `.pin` marker listing the regions that need them and are skipped by LRU eviction and
  never expire; `delete_region` unpins (tiles stay cached but become evictable).
- **Settings parity** commands: `UpdateEnergyParams`, `UpdateReportSettings`,
  `AddCalendarFeed`, `UpdateCalendarFeed`, `DeleteCalendarFeed`, `ReorderCalendarFeeds`, with
  `energy_settings()`, `report_settings()` and `calendar_feeds()` reads. Device listing and
  revoking stay on the web (Settings > Devices); the desktop adds no server routes for them.
- **Logbook "Shared with me"**: `Store::shared_trips()` returns Trips owned by someone else,
  grouped into `edit` and `view`.
- Calendar display zone is a parameter (`Zone`, an `rrule::Tz`): pass `Tz::LOCAL` in the app.
  Windows zone names are mapped for a few common zones; unknown `TZID`s are treated as floating.
- `geocode` calls a server route that answers 501 today and maps to `NotImplemented`.

## Devices (milestone 6)

**Send to a GPS.** A volume under `/Volumes` that contains `Garmin/GPX` (any letter case) is a
GPS. While a Trip has a route, its route options menu lists "Send to <volume name>" for each one.
The file is `gpx_filename(trip name)` holding `build_gpx` output, the same bytes as Export GPX. It
is written to a hidden temporary file in `Garmin/GPX/` and renamed into place, so the device never
sees a partial file. A file with the same name and different content is never replaced: the new
file is named `name (2).gpx`, `name (3).gpx`, and so on (identical content is reused). Success shows
the toast "Sent to <volume>"; errors show inline. Plugging in or ejecting a unit is picked up by a
`notify` watch on `/Volumes`; Settings lists the detected GPS volumes.

**MTP devices are not supported.** Phones and newer Garmin units in MTP mode do not mount as
volumes, so they never appear. Use mass-storage mode, or export the GPX and copy it with another tool.

**Watched folder.** Settings, Devices, "Watched folder" (Choose…, Clear) sets one folder, for
example Downloads. It is a per-device preference stored in `<data dir>/devices.json`, never
synced. The folder (not its subfolders) is watched with `notify`; a new or changed `.gpx` or `.fit`
file is listed after 1 second without further events and a stable size. Partial downloads
(`.crdownload`, `.download`, `.part`) and hidden files are ignored. The 20 most recent files (name,
size, modified time) appear at the top of the Attach GPX sheet, which route options, Attach GPX
opens; click one to attach it. "Choose file…" below opens the file picker. Files already in the
folder when it is chosen are listed too.

**FIT import.** A `.fit` file (chosen, clicked in the sheet or dropped on the window) attaches
like a GPX. `record` positions (semicircles converted to degrees, `enhanced_altitude` or
`altitude` in metres) become route points; `course_point`, then `lap` positions are used only if
there are no records. `strider-core::gpx::route_stats` holds the distance, gain and 800-point
downsampling shared with `parse_gpx`. The route is attached as an equivalent GPX track (7 decimal
positions, 2 decimal elevations, named `<file>.gpx`). A damaged file or one with fewer than two
positions is an inline error and changes nothing.

Tests use temp directories only (`/Volumes` and the home folder are never read): volume detection,
collision naming and atomic send; watcher debounce, partial-file and growing-file handling; FIT
parsing of files generated by the test encoder in `strider-devices::testing` (also exposed to the
app tests by the `test-support` feature); and headless app tests for send, the watched folder and
FIT attach.

## Distribution (milestone 5b)

Strider ships as a signed `Strider.app` in a zip, with a self-hosted, ed25519-signed
manifest for updates (ADR 0006). All tasks run from `desktop/` as `cargo xtask <task>`.

| Task | What it does |
| --- | --- |
| `bundle [--release]` | Builds `strider` and assembles `target/bundle/Strider.app`: `Info.plist` (bundle id `dev.thaiv.strider`, version from `strider-app`'s `Cargo.toml`, macOS 13.0+, `strider://` scheme, GPX viewer), `AppIcon.icns` rendered from `app/icon.svg`, and `assets/`. |
| `sign` | Signs the binary then the bundle with the hardened runtime and `packaging/Strider.entitlements` (network client, user-selected files; no JIT), then runs `codesign --verify --deep --strict`. |
| `notarize` | Zips the bundle, submits it with `notarytool --wait`, staples and checks with `spctl`. |
| `release` | `bundle --release`, `sign`, `notarize` when configured, then `target/release-artifacts/Strider-<version>.zip` and `manifest.json`. |
| `keygen` | Prints a new update key pair (base64) to stdout. |

Bump `version` in `crates/strider-app/Cargo.toml` before every release; the updater only
installs a strictly newer semver.

### Environment variables

| Variable | Used by | Meaning |
| --- | --- | --- |
| `STRIDER_UPDATE_URL` | `bundle`, `release` | HTTPS URL of `manifest.json`, baked into the build (`StriderUpdateURL`). Default: `https://github.com/thaiv28/strider/releases/latest/download/manifest.json`. Updates stay off in builds without `STRIDER_UPDATE_PUBKEY`. |
| `STRIDER_UPDATE_PUBKEY` | `bundle`, `release` | Base64 ed25519 public key, baked in (`StriderUpdatePublicKey`). Default: `packaging/update-public-key.txt`. |
| `STRIDER_SIGN_IDENTITY` | `sign` | Developer ID Application identity name. Unset: ad-hoc signing (`--sign -`, no timestamp) and a one-line notice. |
| `STRIDER_NOTARY_PROFILE` | `notarize`, `release` | A `notarytool` keychain profile (`xcrun notarytool store-credentials <name>`). Unset: notarization is skipped and the missing variable is printed. |
| `STRIDER_UPDATE_SIGNING_KEY` | `release` | Base64 32-byte seed. Default: the login Keychain item `dev.thaiv.strider.update-signing` (where the current seed lives). Neither: no `manifest.json` is written. Must match `STRIDER_UPDATE_PUBKEY` when both are set. Keep it out of the repo. |
| `STRIDER_RELEASE_URL` | `release` | Zip URL for the manifest. Default: the manifest URL with its file name replaced by `Strider-<version>.zip`. |
| `STRIDER_RELEASE_NOTES` | `release` | Text shown in the update Sheet. |

### Generate keys

```sh
cargo xtask keygen
```

Store `STRIDER_UPDATE_SIGNING_KEY` in a password manager or CI secret; it is never logged by
any other task. Put `STRIDER_UPDATE_PUBKEY` in the environment of every `bundle`/`release`.
Losing or rotating the key means shipping one more build by hand: installed apps only trust
the key baked into them.

### Cut a release

Releases are GitHub releases on this repository, built by
`.github/workflows/desktop-release.yml`:

1. Bump `version` in `desktop/crates/strider-app/Cargo.toml` and merge it to `main`.
2. Tag that commit `desktop-v<version>` and push the tag.

The workflow checks the tag matches the version, runs `cargo test --workspace` on an Apple
silicon runner, runs `cargo xtask release` with the `STRIDER_UPDATE_SIGNING_KEY` repository
secret, and creates the release with `Strider-<version>.zip` and `manifest.json`, marked
latest. Installed apps read `releases/latest/download/manifest.json`, so the newest desktop
release must stay "latest"; mark any non-desktop release otherwise. Each manifest's `url`
points at its own tag's zip. To build locally instead, run `cargo xtask release` (the seed is
read from the Keychain item `dev.thaiv.strider.update-signing`).

Signing with a Developer ID and notarizing will need `STRIDER_SIGN_IDENTITY` and
`STRIDER_NOTARY_PROFILE` plus the certificate on the runner; until then releases are ad-hoc
signed. The manifest is
`{ version, url, sha256, size, signature, notes, minimum_os }`; `signature` is ed25519 over
`"<version>\n<sha256>"`.

### How the app updates

With no `StriderUpdateURL`/`StriderUpdatePublicKey` in the running bundle's Info.plist (or
when not running from a `.app`, as with `cargo run`) updates are off and the Strider menu
shows a disabled "Updates are not configured". Otherwise the primary window checks 30 s
after launch and every 24 h, and on Strider > Check for Updates…. It fetches the manifest,
verifies the signature, and compares semver. A newer release shows "Strider <v> is available"
with the notes and Install and Relaunch / Later (a background check only shows a toast). Install downloads the
zip to a temp directory, checks size and SHA-256, unzips with `ditto`, runs
`codesign --verify --deep --strict`, and requires the same bundle id and Team ID as the
running app (ad-hoc builds compare the bundle id only). A detached helper then waits for the
app to quit, swaps the bundles, relaunches with `open -n` and deletes the old copy.

### Gatekeeper and ad-hoc builds

Without `STRIDER_SIGN_IDENTITY` the bundle is ad-hoc signed: it runs on the machine that
built it, but on another Mac Gatekeeper refuses it until the user right-clicks the app and
chooses Open (then Open again), or approves it in System Settings > Privacy & Security.
Only Developer ID-signed, notarized builds open with a double-click. Updates between ad-hoc
builds work, because the bundle id check stands in for the Team ID.

Not covered by tests: notarization and a real signed update need an Apple Developer ID and an
update host; the unit tests exercise the manifest, signature, hash and identity logic with
fake transports.
