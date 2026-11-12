---
version: 1
slug: "components-landing-landing-page-tsx"
primary_target: "components/landing/landing-page.tsx"
related_targets: ["app/login/page.tsx"]
---

Scope: public landing page `/` and sign-in `/login`. Mode: Persuade (for the owner's own front door; minimal pitch).

Audience and job: Thai, and the occasional invited trip partner, arriving to sign in. Action: Sign in with Google. Proof: Strider's own UI, recreated at full fidelity with one sample trip (The Enchantments, Sep 24–27, 18.2 mi, 4,550 ft, 3 days). Constraints: DESIGN.md (Topo) tokens, fonts and primitives only; no feature copy.

## Direction contract

THESIS: The page is one trip, shown the way the app shows it. It refuses the split hero over feature cards and every sentence that describes a feature.
OWN-WORLD: The forest header band with faint contour lines fills the page. A canvas-ground trip card with a white header, accent-filled section chips, panel cards inside, the route in orange, base/consumable/worn weight colors, tabular readouts in Bricolage.
STORY: The visitor sees a real trip's route, pack and food, understands that this is where the whole trip gets planned, and signs in.
FIRST VIEWPORT: The left column (~360px) holds the wordmark at the top, "Plan your backpacking trip in one place." in display type at the vertical middle with the Google button directly under it and no subcopy, and Privacy at the bottom. The right side is a large trip card at ~65% width, full height, with chips for Route, Gear and Food.
FORM: One trip, three views; #2 on the ordered list; seed:6dedbd993ad705c7. Signature interaction: the chips switch panels exactly like the app's trip sections, with a short crossfade. On a phone the card stacks under the headline and chips stay sticky.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

Unresolved: whether the sign-in page reuses the landing's left column or remains a centered card.
