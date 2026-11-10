"use client";

import Link from "next/link";
import { ArrowRight, Backpack, CloudSun, Map, Route, Share2, Utensils } from "lucide-react";

const features = [
  { icon: Route, kicker: "Route", title: "Shape the whole trip", copy: "Import a GPX, place campsites, split mileage by day, and keep permits and access details beside the map." },
  { icon: Backpack, kicker: "Pack", title: "Know every ounce", copy: "Build reusable loadouts, compare base and worn weight, and turn the final list into a live packing checklist." },
  { icon: Utensils, kicker: "Fuel", title: "Plan food that adds up", copy: "Organize meals by day, track calories and carried weight, size stove fuel, and generate the grocery list." },
  { icon: CloudSun, kicker: "Conditions", title: "Prepare for the mountain", copy: "See trip-specific forecasts, climate context, snow, radar, water sources, and route elevation in one place." },
];

export function LandingPage() {
  return (
    <div className="landing min-h-screen bg-canvas text-ink">
      <header className="absolute inset-x-0 top-0 z-30 text-headerink">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-5 py-5 sm:px-8">
          <a href="#top" className="inline-flex min-h-11 items-center font-display text-xl font-bold">Strider</a>
          <div className="flex items-center gap-4">
            <a href="#features" className="hidden min-h-11 items-center text-sm text-headermuted transition hover:text-headerink sm:inline-flex">Features</a>
            <Link href="/login" className="inline-flex min-h-11 items-center rounded-xl bg-headeractive px-4 py-2 text-sm font-medium transition hover:bg-headerline">Log in</Link>
          </div>
        </div>
      </header>

      <section id="top" className="relative isolate flex items-center overflow-hidden bg-header pt-28 text-headerink sm:pt-36">
        <TopoHero />
        <div className="relative z-10 mx-auto grid w-full max-w-7xl items-center gap-12 px-5 pb-24 sm:px-8 lg:grid-cols-2 lg:gap-10">
          <div className="min-w-0 max-w-2xl">
            <div className="mb-6 inline-flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.14em] text-headermuted">
              <span className="h-1.5 w-1.5 rounded-full bg-route" /> Your trailhead for every trip
            </div>
            <h1 className="font-display text-5xl font-bold leading-none tracking-[-0.04em] sm:text-7xl lg:text-[5.25rem]">
              Plan farther.<br /><span className="text-headermuted">Pack smarter.</span>
            </h1>
            <p className="mt-6 max-w-xl text-lg leading-relaxed text-headermuted sm:text-xl">
              Routes, gear, meals, weather, logistics, and the final trip plan—one calm workspace from first idea to trailhead.
            </p>
            <div className="mt-8 flex flex-wrap gap-3">
              <Link href="/login" className="landing-header-button group inline-flex min-h-11 items-center gap-2 rounded-xl bg-headerink px-5 py-3 font-semibold text-headeractionink transition">Open Strider <ArrowRight size={17} className="transition group-hover:translate-x-1" /></Link>
              <a href="#story" className="inline-flex min-h-11 items-center rounded-xl px-5 py-3 font-medium text-headerink transition hover:bg-headeractive">See how it works</a>
            </div>
          </div>
          <ProductFrame />
        </div>
        <div className="absolute bottom-7 left-1/2 z-10 -translate-x-1/2 text-center text-[0.65rem] uppercase tracking-[0.26em] text-headermuted">Scroll to explore<br /><span className="mt-2 inline-block">↓</span></div>
      </section>

      <section id="story" className="relative bg-canvas py-20 text-ink sm:py-28">
        <div className="mx-auto max-w-7xl px-5 sm:px-8">
          <div className="grid gap-6 lg:grid-cols-[0.8fr_1.2fr] lg:items-end">
            <div><div className="text-xs font-semibold uppercase tracking-[0.2em] text-muted">One source of truth</div><h2 className="font-display mt-4 text-4xl font-bold leading-tight tracking-tight sm:text-6xl">The plan stays connected.</h2></div>
            <p className="max-w-2xl text-lg leading-relaxed text-muted">Move a campsite and the itinerary changes. Add a meal and carried food weight follows. Update the packing list and the group sees it at the same link. Strider keeps the details working together.</p>
          </div>
          <div className="mt-16 grid gap-4 md:grid-cols-3">
            <StoryCard icon={Map} number="01" title="Build the route" copy="Trace the objective from trailhead to camp, with daily mileage and elevation ready before you leave." />
            <StoryCard icon={Backpack} number="02" title="Balance the load" copy="See base, worn, food, water, and fuel weight together—and exactly what changed the total." />
            <StoryCard icon={Share2} number="03" title="Bring everyone in" copy="Print it, generate the pre-trip brief, or send one live view-only link to the group." />
          </div>
        </div>
      </section>

      <section id="features" className="bg-canvas pb-20 pt-4 text-ink sm:pb-28">
        <div className="mx-auto max-w-6xl px-5 sm:px-8">
          <div className="mx-auto max-w-3xl text-center"><div className="text-xs font-semibold uppercase tracking-[0.2em] text-muted">From idea to trailhead</div><h2 className="font-display mt-4 text-4xl font-bold tracking-tight sm:text-6xl">Everything the trip needs.<br />Nothing it doesn’t.</h2></div>
          <div className="mt-12 grid gap-4 md:grid-cols-2">
            {features.map(({ icon: Icon, kicker, title, copy }, index) => (
              <div key={title} className="grid min-w-0 gap-4 rounded-2xl bg-panel p-6 sm:p-8">
                <div className="flex h-12 items-center text-signal"><Icon size={21} /></div>
                <div><div className="text-xs font-semibold uppercase tracking-[0.18em] text-muted">{String(index + 1).padStart(2, "0")} · {kicker}</div><h3 className="font-display mt-1 text-2xl font-bold sm:text-3xl">{title}</h3></div>
                <p className="leading-relaxed text-muted">{copy}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="bg-canvas px-5 pb-20 text-center sm:px-8 sm:pb-28">
        <div className="relative mx-auto max-w-5xl rounded-2xl bg-panel px-5 py-16 sm:px-10 sm:py-20"><div className="text-xs font-semibold uppercase tracking-[0.2em] text-muted">The trail starts here</div><h2 className="font-display mt-5 text-4xl font-bold leading-none tracking-tight sm:text-7xl">Make the next trip<br />the best-planned one.</h2><Link href="/login" className="group mt-9 inline-flex min-h-11 items-center gap-2 rounded-xl bg-accent px-6 py-3 font-semibold text-accentink transition hover:opacity-90">Enter Strider <ArrowRight size={17} className="transition group-hover:translate-x-1" /></Link></div>
      </section>
      <footer className="mx-auto flex max-w-7xl flex-col gap-3 bg-canvas px-5 py-8 text-sm text-muted sm:flex-row sm:items-center sm:justify-between sm:px-8"><span className="font-display font-bold text-ink">Strider</span><div className="flex flex-wrap items-center gap-x-5 gap-y-2"><span>Built for the miles before the miles.</span><Link href="/privacy" className="inline-flex min-h-11 items-center text-muted underline-offset-4 transition hover:text-ink hover:underline">Privacy</Link></div></footer>
    </div>
  );
}

function TopoHero() {
  return <svg aria-hidden="true" className="pointer-events-none absolute inset-0 h-full w-full" viewBox="0 0 1200 800" preserveAspectRatio="xMidYMid slice"><g className="landing-contours" fill="none" stroke="var(--header-line)" strokeWidth="1"><path d="M-50 180C160 20 280 330 500 170S820 40 1250 230"/><path d="M-70 230C150 70 300 380 520 215S850 80 1270 275"/><path d="M-90 285C140 115 320 430 545 265S870 125 1290 325"/><path d="M-80 590C110 430 300 700 510 555S850 405 1280 610"/><path d="M-80 645C120 480 315 750 530 605S875 455 1290 665"/></g></svg>;
}

function ProductFrame() {
  return (
    <div aria-hidden="true" className="relative mx-auto w-full min-w-0 max-w-2xl overflow-hidden rounded-2xl bg-panel text-ink">
      <div className="px-5 pb-5 pt-6 sm:px-7">
        <div className="text-[0.6rem] font-semibold tracking-[0.15em] text-muted">STRIDER / ENCHANTMENTS</div>
        <div className="mt-5 text-[0.6rem] font-semibold uppercase tracking-[0.18em] text-signal">Next objective</div>
        <div className="font-display mt-1 text-2xl font-bold sm:text-3xl">The Enchantments</div>
        <div className="mt-1 text-xs text-muted">Alpine Lakes Wilderness · Sep 24–27</div>
      </div>
      <svg className="block aspect-[5/3] w-full bg-map" viewBox="0 0 500 300" fill="none">
        <g stroke="var(--contour)" strokeWidth="1.2">
          {Array.from({ length: 12 }, (_, i) => (
            <path key={i} d={`M-40 ${i * 28 - 80}C65 ${i * 23 - 30} 110 ${i * 20 + 120} 240 ${i * 25 + 50}S390 ${i * 24 - 70} 540 ${i * 27 + 20}`} />
          ))}
          {[28, 48, 68, 88, 108].map((r) => <ellipse key={r} cx="375" cy="235" rx={r} ry={r * .6} transform="rotate(-20 375 235)" />)}
        </g>
        <path d="M280 0C260 35 300 45 285 75S310 110 335 85S310 25 340 0" fill="var(--lake)" />
        <path d="M38 245C100 218 120 125 182 155S286 65 336 98S415 68 467 38" stroke="var(--route)" strokeWidth="4" strokeLinecap="round" />
        {[[38, 245], [182, 155], [336, 98], [467, 38]].map(([x, y], i) => (
          <g key={i}>
            <circle cx={x} cy={y} r="12" fill="var(--panel)" stroke="var(--route)" strokeWidth="2.5" />
            <text x={x} y={y + 4} textAnchor="middle" fill="var(--ink)" stroke="none" fontSize="12" fontWeight="700">{i + 1}</text>
          </g>
        ))}
      </svg>
      <div className="px-5 py-5 sm:px-7">
        <svg className="block h-16 w-full" viewBox="0 0 500 80" preserveAspectRatio="none" fill="none">
          <path d="M0 70L35 59 75 63 125 35 155 43 210 12 240 25 285 5 325 22 355 18 405 45 445 38 500 65V80H0Z" fill="var(--elevation)" />
          <path d="M0 70L35 59 75 63 125 35 155 43 210 12 240 25 285 5 325 22 355 18 405 45 445 38 500 65" stroke="var(--route)" strokeWidth="2" vectorEffect="non-scaling-stroke" />
        </svg>
        <div className="mt-5 grid grid-cols-3 gap-2">
          <MiniMetric label="DISTANCE" value="18.2 mi" /><MiniMetric label="GAIN" value="4,550 ft" /><MiniMetric label="BASE" value="12 lb 8 oz" />
        </div>
        <div className="mt-5 flex flex-wrap gap-x-4 gap-y-2 text-[0.65rem] font-medium text-muted"><span>✓ ROUTE</span><span>✓ FOOD</span><span>PACKING 82%</span></div>
      </div>
    </div>
  );
}

function MiniMetric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><div className="text-[0.55rem] font-semibold tracking-wider text-muted">{label}</div><div className="mt-1 font-mono text-[0.65rem] font-semibold sm:text-xs">{value}</div></div>;
}

function StoryCard({ icon: Icon, number, title, copy }: { icon: typeof Map; number: string; title: string; copy: string }) {
  return <article className="min-w-0 rounded-2xl bg-panel p-6 text-ink sm:p-8"><div className="flex items-center justify-between"><Icon size={24} className="text-signal" /><span className="font-mono text-xs text-muted">{number}</span></div><h3 className="font-display mt-12 text-3xl font-bold">{title}</h3><p className="mt-3 leading-relaxed text-muted">{copy}</p></article>;
}
