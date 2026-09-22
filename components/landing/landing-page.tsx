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
    <div className="landing min-h-screen overflow-hidden bg-[#101713] text-[#f4f1e8]">
      <header className="absolute inset-x-0 top-0 z-30">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-5 py-5 sm:px-8">
          <a href="#top" className="font-display text-lg font-bold tracking-[0.16em]">STRIDER</a>
          <div className="flex items-center gap-4">
            <a href="#features" className="hidden text-sm text-[#c8c8b9] transition hover:text-white sm:block">Features</a>
            <Link href="/login" className="rounded-full border border-white/20 bg-white/10 px-4 py-2 text-sm font-medium backdrop-blur transition hover:bg-white/15">Enter Strider</Link>
          </div>
        </div>
      </header>

      <section id="top" className="relative flex min-h-[94svh] items-center pt-24">
        <div className="landing-glow absolute inset-0" />
        <TopoHero />
        <div className="relative z-10 mx-auto grid w-full max-w-7xl items-center gap-12 px-5 pb-16 sm:px-8 lg:grid-cols-[0.88fr_1.12fr] lg:gap-10">
          <div className="max-w-2xl">
            <div className="mb-5 inline-flex items-center gap-2 rounded-full border border-[#8bd3b2]/25 bg-[#8bd3b2]/10 px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.18em] text-[#a9e0c6]">
              <span className="h-1.5 w-1.5 rounded-full bg-[#8bd3b2]" /> Your trailhead for every trip
            </div>
            <h1 className="font-display text-5xl font-bold leading-[0.96] tracking-[-0.045em] sm:text-7xl lg:text-[5.8rem]">
              Plan farther.<br /><span className="text-[#91cfad]">Pack smarter.</span>
            </h1>
            <p className="mt-6 max-w-xl text-lg leading-relaxed text-[#c8c8b9] sm:text-xl">
              Routes, gear, meals, weather, logistics, and the final trip plan—one calm workspace from first idea to trailhead.
            </p>
            <div className="mt-8 flex flex-wrap gap-3">
              <Link href="/login" className="group inline-flex items-center gap-2 rounded-full bg-[#d7f2df] px-5 py-3 font-semibold text-[#122219] transition hover:bg-white">Open Strider <ArrowRight size={17} className="transition group-hover:translate-x-1" /></Link>
              <a href="#story" className="inline-flex items-center rounded-full border border-white/20 px-5 py-3 font-medium text-[#e5e2d8] transition hover:bg-white/10">See how it works</a>
            </div>
          </div>
          <ProductFrame />
        </div>
        <div className="absolute bottom-7 left-1/2 z-10 -translate-x-1/2 text-center text-[0.65rem] uppercase tracking-[0.26em] text-white/45">Scroll to explore<br /><span className="mt-2 inline-block animate-bounce">↓</span></div>
      </section>

      <section id="story" className="relative border-y border-white/10 bg-[#eae5d8] py-24 text-[#272a25] sm:py-32">
        <div className="mx-auto max-w-7xl px-5 sm:px-8">
          <div className="grid gap-12 lg:grid-cols-[0.8fr_1.2fr] lg:items-end">
            <div><div className="text-xs font-semibold uppercase tracking-[0.2em] text-[#397461]">One source of truth</div><h2 className="font-display mt-4 text-4xl font-bold leading-tight tracking-tight sm:text-6xl">The plan stays connected.</h2></div>
            <p className="max-w-2xl text-lg leading-relaxed text-[#5b5d56]">Move a campsite and the itinerary changes. Add a meal and carried food weight follows. Update the packing list and the group sees it at the same link. Strider keeps the details working together.</p>
          </div>
          <div className="mt-16 grid gap-4 md:grid-cols-3">
            <StoryCard icon={Map} number="01" title="Build the route" copy="Trace the objective from trailhead to camp, with daily mileage and elevation ready before you leave." accent="bg-[#315f52]" />
            <StoryCard icon={Backpack} number="02" title="Balance the load" copy="See base, worn, food, water, and fuel weight together—and exactly what changed the total." accent="bg-[#4b6845]" />
            <StoryCard icon={Share2} number="03" title="Bring everyone in" copy="Print it, generate the pre-trip brief, or send one live view-only link to the group." accent="bg-[#526d82]" />
          </div>
        </div>
      </section>

      <section id="features" className="bg-[#f4f0e7] py-24 text-[#292520] sm:py-32">
        <div className="mx-auto max-w-6xl px-5 sm:px-8">
          <div className="mx-auto max-w-3xl text-center"><div className="text-xs font-semibold uppercase tracking-[0.2em] text-[#397461]">From idea to trailhead</div><h2 className="font-display mt-4 text-4xl font-bold tracking-tight sm:text-6xl">Everything the trip needs.<br />Nothing it doesn’t.</h2></div>
          <div className="mt-16 divide-y divide-[#292520]/10 border-y border-[#292520]/10">
            {features.map(({ icon: Icon, kicker, title, copy }, index) => (
              <div key={title} className="group grid gap-4 py-8 sm:grid-cols-[4rem_0.75fr_1.25fr] sm:items-center sm:gap-8 sm:py-10">
                <div className="flex h-12 w-12 items-center justify-center rounded-full border border-[#397461]/20 bg-[#397461]/10 text-[#397461] transition group-hover:scale-110"><Icon size={21} /></div>
                <div><div className="text-xs font-semibold uppercase tracking-[0.18em] text-[#717268]">{String(index + 1).padStart(2, "0")} · {kicker}</div><h3 className="font-display mt-1 text-2xl font-bold sm:text-3xl">{title}</h3></div>
                <p className="leading-relaxed text-[#65665f]">{copy}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="relative overflow-hidden bg-[#142019] px-5 py-28 text-center sm:px-8 sm:py-36">
        <div className="landing-rings absolute inset-0 opacity-30" />
        <div className="relative z-10 mx-auto max-w-3xl"><div className="text-xs font-semibold uppercase tracking-[0.2em] text-[#91cfad]">The trail starts here</div><h2 className="font-display mt-5 text-5xl font-bold leading-none tracking-tight sm:text-7xl">Make the next trip<br />the best-planned one.</h2><Link href="/login" className="group mt-9 inline-flex items-center gap-2 rounded-full bg-[#d7f2df] px-6 py-3 font-semibold text-[#122219] transition hover:bg-white">Enter Strider <ArrowRight size={17} className="transition group-hover:translate-x-1" /></Link></div>
      </section>
      <footer className="flex flex-col gap-3 border-t border-white/10 bg-[#101713] px-5 py-8 text-sm text-white/45 sm:flex-row sm:items-center sm:justify-between sm:px-8"><span className="font-display font-bold tracking-[0.14em] text-white/75">STRIDER</span><div className="flex flex-wrap items-center gap-x-5 gap-y-2"><span>Built for the miles before the miles.</span><Link href="/privacy" className="text-white/65 underline-offset-4 transition hover:text-white hover:underline">Privacy</Link></div></footer>
    </div>
  );
}

function TopoHero() {
  return <svg aria-hidden="true" className="absolute inset-0 h-full w-full opacity-30" viewBox="0 0 1200 800" preserveAspectRatio="xMidYMid slice"><g className="landing-contours" fill="none" stroke="#7faf93" strokeWidth="1"><path d="M-50 180C160 20 280 330 500 170S820 40 1250 230"/><path d="M-70 230C150 70 300 380 520 215S850 80 1270 275"/><path d="M-90 285C140 115 320 430 545 265S870 125 1290 325"/><path d="M-80 590C110 430 300 700 510 555S850 405 1280 610"/><path d="M-80 645C120 480 315 750 530 605S875 455 1290 665"/></g><path className="landing-route-line" d="M80 650C250 590 225 420 415 430S610 290 735 345S950 210 1120 145" fill="none" stroke="#d7f2df" strokeWidth="3" strokeLinecap="round"/><circle cx="80" cy="650" r="7" fill="#d7f2df"/><circle className="landing-route-dot" cx="1120" cy="145" r="8" fill="#91cfad"/></svg>;
}

function ProductFrame() {
  return <div className="landing-float relative mx-auto w-full max-w-2xl"><div className="absolute -inset-10 rounded-full bg-[#6bb88f]/10 blur-3xl"/><div className="relative overflow-hidden rounded-2xl border border-white/15 bg-[#f4f0e7] shadow-2xl shadow-black/40"><div className="flex h-9 items-center gap-1.5 border-b border-black/10 bg-[#ded8ca] px-4"><span className="h-2.5 w-2.5 rounded-full bg-[#c27464]"/><span className="h-2.5 w-2.5 rounded-full bg-[#c7a957]"/><span className="h-2.5 w-2.5 rounded-full bg-[#6c9b72]"/><span className="ml-3 text-[0.6rem] font-semibold tracking-[0.15em] text-black/40">STRIDER / ENCHANTMENTS</span></div><div className="grid min-h-[28rem] grid-cols-[4rem_1fr] text-[#292520] sm:grid-cols-[8rem_1fr]"><div className="border-r border-black/10 bg-[#e7e1d4] p-3"><div className="font-display text-xs font-bold tracking-wide">STRIDER</div><div className="mt-8 space-y-3 text-[0.6rem] text-black/45"><div className="font-semibold text-[#1f7a70]">BASECAMP</div><div>GEAR</div><div>FOOD</div><div>TRIPS</div></div></div><div className="p-5 sm:p-7"><div className="text-[0.58rem] font-semibold uppercase tracking-[0.18em] text-[#1f7a70]">Next objective</div><div className="font-display mt-1 text-2xl font-bold sm:text-3xl">The Enchantments</div><div className="mt-1 text-xs text-black/45">Alpine Lakes Wilderness · Sep 24–27</div><div className="mt-5 grid grid-cols-3 gap-2"><MiniMetric label="DISTANCE" value="18.2 mi"/><MiniMetric label="GAIN" value="4,550 ft"/><MiniMetric label="BASE" value="12 lb 8 oz"/></div><div className="relative mt-5 h-40 overflow-hidden rounded-lg bg-[#cad6c2]"><svg className="h-full w-full" viewBox="0 0 500 180"><g fill="none" stroke="#667c62" strokeWidth="0.7" opacity=".45"><path d="M-10 60C70 20 130 100 210 60S350 20 520 80"/><path d="M-10 80C70 40 130 120 215 80S360 40 520 100"/><path d="M-10 105C80 65 150 145 235 105S380 65 520 125"/></g><path d="M38 142C100 115 120 62 182 92S286 42 336 75S415 45 467 28" fill="none" stroke="#1f7a70" strokeWidth="4" strokeLinecap="round"/><g fill="#f7f2e7" stroke="#1f7a70" strokeWidth="3"><circle cx="38" cy="142" r="6"/><circle cx="182" cy="92" r="6"/><circle cx="336" cy="75" r="6"/><circle cx="467" cy="28" r="6"/></g></svg></div><div className="mt-5 flex flex-wrap gap-2 text-[0.65rem]"><span className="rounded-full bg-[#4e6e47]/15 px-2.5 py-1 text-[#4e6e47]">✓ ROUTE</span><span className="rounded-full bg-[#4e7b96]/15 px-2.5 py-1 text-[#4e7b96]">✓ FOOD</span><span className="rounded-full bg-[#a08a4e]/15 px-2.5 py-1 text-[#7b682e]">PACKING 82%</span></div></div></div></div></div>;
}

function MiniMetric({ label, value }: { label: string; value: string }) { return <div className="rounded-md border border-black/10 bg-white/35 p-2"><div className="text-[0.5rem] font-semibold tracking-wider text-black/40">{label}</div><div className="mt-1 font-mono text-[0.68rem] font-semibold sm:text-xs">{value}</div></div>; }
function StoryCard({ icon: Icon, number, title, copy, accent }: { icon: typeof Map; number: string; title: string; copy: string; accent: string }) { return <article className={`${accent} min-h-80 rounded-2xl p-6 text-[#f5f1e8] transition duration-500 hover:-translate-y-2 sm:p-8`}><div className="flex items-center justify-between"><Icon size={24}/><span className="font-mono text-xs text-white/45">{number}</span></div><h3 className="font-display mt-24 text-3xl font-bold">{title}</h3><p className="mt-3 leading-relaxed text-white/70">{copy}</p></article>; }
