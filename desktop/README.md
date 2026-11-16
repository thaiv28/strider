# Strider Desktop — milestone 0

A native Rust/GPUI evaluation of the web app's Topo Gear inventory, using a
local 2,000-item SQLite fixture. This is a toolkit spike, with no web service,
sync, authentication, production data, or implementation of the other areas.

Run from `desktop/` so rustup selects the pinned toolchain:

```sh
cargo run -p strider-app --release
cargo run -p strider-app --release -- --reset
# Everything needed is cached; offline development:
cargo run --offline -p strider-app -- --reset
```

The database is `$HOME/Library/Application Support/Strider/spike/workspace.db`.
The app creates its parent directories, enables foreign keys and WAL, and applies
schema version 1 using `PRAGMA user_version`. `--reset` deletes this fixture file
and reseeds it. Fixture values and valid UUIDv7 identities are deterministic;
the identities use a fixed fixture timestamp and monotonic payloads. Undo history
lives only in memory and disappears at quit.

The pinned pair is `gpui-pre`/`gpui-pre-platform` **0.3.12**, used with
`gpui-component` **0.9.1**. Upgrade them together. The platform's `runtime_shaders`
feature compiles Metal shaders at launch, allowing builds with Apple's Command
Line Tools instead of requiring the full Xcode shader compiler. `font-kit`
supports font discovery; the UI embeds Instrument Sans and Bricolage Grotesque.
Startup uses `gpui_platform::application()` and initializes gpui-component before
wrapping the application view in its `Root`.

| Crate | Responsibility |
| --- | --- |
| `strider-core` | Grams/cents and web-compatible weight formatting; no dependencies |
| `strider-store` | SQLite schema, deterministic fixture, queries, transactional commands, undo/redo |
| `strider-ui` | Generated Topo colors, embedded fonts, primitives, icons and contour drawing |
| `strider-app` (`strider`) | Window, navigation, virtual Gear list, actions, editor and drag/drop |

`strider-ui/build.rs` reads the Topo light/dark blocks in `app/globals.css` and
fails on property mismatches. Category colors are the ported web table. The
Rust design guard rejects raw colors and literal font families outside the UI
implementation. No web stylesheet is changed.

## Interaction

Click a row to select it; double-click or use its more button to edit. Expand a
kit with its chevron. Filter by item or category name and Current/Retired/All;
Wishlist items never enter the inventory. Drag a grip to insert before another
row; drop on a category header to append to it. Dragging is disabled while the
text filter is nonempty. All commands reload groups and current-item totals.

| Shortcut | Action |
| --- | --- |
| ↑ / ↓ | Select previous/next item and reveal it |
| Return | Edit selected item |
| ⌘F | Focus Filter gear |
| Escape | Leave filter focus / close the edit Sheet |
| ⌫ | Delete selected item, without confirmation |
| ⌘Z / ⇧⌘Z | Undo / redo inventory command |
| ⌘Q | Quit |

The menu bar exposes these actions and View → Appearance → Light / Dark /
System. Text inputs retain their native text editing shortcuts. Inventory
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

Tests include hand-computed TypeScript formatting outputs (including exact
quarter-ounce decimal ties), SQLite fixture and command invariants, the Rust
design guard, and a GPUI headless Gear view test. The headless test loads 2,000
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

One `rusqlite::Connection` belongs to `Store`; queries and commands run
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

The implementation-session launch check reached the executable but the sandbox
denied access to the default Application Support directory. Startup printed the
IO error and exited without a panic; the required five-second native launch
check therefore did **not** pass in this environment. Run the command on the
host with access to that directory before judging native startup or visuals.
