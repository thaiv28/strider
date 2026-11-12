"use client";

import { useState } from "react";
import { Badge, Chip } from "@/components/ui";
import { categoryColor } from "@/lib/categories";

// Static illustrative sample only; no private trip data is loaded.
export function TripPreview() {
  const [view, setView] = useState<"route" | "gear" | "food">("route");
  return (
    <section aria-label="Sample trip: The Enchantments" className="order-2 flex h-auto min-w-0 flex-col self-center overflow-clip rounded-[20px] bg-canvas text-ink shadow-2xl desk:h-[min(780px,calc(100dvh-80px))] desk:overflow-hidden">
      <div className="sticky top-0 z-10 rounded-t-[20px] bg-panel px-4 pt-4 sm:static sm:px-6 sm:pt-5">
        <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
          <h2 className="font-display text-[26px] font-bold leading-[1.1] sm:text-[34px]">The Enchantments</h2>
          <span className="text-[13px] text-muted">Sep 24–26 · Planned</span>
        </div>
        <div className="flex gap-2 overflow-x-auto py-4" role="group" aria-label="Trip sections">
          {(["route", "gear", "food"] as const).map((section) => <Chip key={section} selected={view === section} onClick={() => setView(section)}>{section === "route" ? "Route" : section === "gear" ? "Gear" : "Food"}</Chip>)}
        </div>
      </div>
      <div className="relative min-h-0 flex-1 desk:overflow-auto">

      {view === "route" && <div key="route" className="grid min-h-full gap-2.5 p-2.5 sm:gap-3.5 sm:p-3.5 animate-[landing-view_.28s_cubic-bezier(.16,1,.3,1)] items-start sm:grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)]">
        <div className="card flex flex-col self-stretch overflow-hidden">
          <svg className="block min-h-[220px] w-full flex-1 bg-map" viewBox="0 -90 600 520" preserveAspectRatio="xMidYMid slice" role="img" aria-label="Route map from Stuart Lake trailhead over Aasgard Pass to Snow Lakes trailhead">
            <g fill="none" stroke="var(--contour)" strokeWidth="1.2">
              <path d="M-20 -70C100 -100 180 10 300 -40S480 -90 620 -50"/><path d="M-20 -15C100 -45 180 65 300 15S480 -35 620 5"/><path d="M-20 370C150 340 280 450 400 400S580 350 620 390"/><path d="M-20 420C150 390 280 500 400 450S580 400 620 440"/>
              <path d="M-20 40C100 10 180 120 300 70S480 20 620 60"/><path d="M-20 95C110 65 200 175 320 125S500 75 620 115"/><path d="M-20 150C120 120 220 230 340 180S520 130 620 170"/><path d="M-20 205C130 175 240 285 360 235S540 185 620 225"/><path d="M-20 260C140 230 260 340 380 290S560 240 620 280"/><path d="M-20 315C150 285 280 395 400 345S580 295 620 335"/>
              <ellipse cx="300" cy="62" rx="40" ry="22"/><ellipse cx="300" cy="62" rx="22" ry="11"/>
            </g>
            <g transform="translate(300 170) scale(.82) translate(-300 -170)"><path d="M210 168c-14 12 4 34 24 28s26-24 6-32-22-2-30 4z" fill="var(--lake)"/>
            <path d="M346 96c-12 8 2 24 18 20s20-16 6-22-16-2-24 2z" fill="var(--lake)"/>
            <path d="M420 150c-10 10 4 22 20 18s18-14 4-20-16-2-24 2z" fill="var(--lake)"/>
            <path d="M48 300C100 270 150 210 222 196S280 120 300 66 340 100 362 112 420 140 450 172 520 210 560 250" fill="none" stroke="var(--route)" strokeWidth="4" strokeLinecap="round" strokeLinejoin="round"/>
            <g fontSize="11" fontWeight="700" textAnchor="middle" fill="var(--ink)">
              <circle cx="222" cy="196" r="10" fill="var(--panel)" stroke="var(--route)" strokeWidth="2.5"/><text x="222" y="200">1</text>
              <circle cx="362" cy="112" r="10" fill="var(--panel)" stroke="var(--route)" strokeWidth="2.5"/><text x="362" y="116">2</text>
            </g>
            <circle cx="48" cy="300" r="5" fill="var(--route)"/><rect x="554" y="244" width="12" height="12" rx="2" fill="var(--route)"/></g>
          </svg>
          <svg className="block h-14 w-full" viewBox="0 0 600 60" preserveAspectRatio="none" aria-hidden="true"><path d="M0 54L60 46 110 30 140 28 190 6 205 3 220 8 260 12 330 30 420 44 600 56V60H0Z" fill="var(--elevation)"/><path d="M0 54L60 46 110 30 140 28 190 6 205 3 220 8 260 12 330 30 420 44 600 56" fill="none" stroke="var(--route)" strokeWidth="2" vectorEffect="non-scaling-stroke"/></svg>
          <div className="flex flex-wrap gap-x-7 gap-y-3 px-[18px] py-3.5">
            <div><span className="block font-display text-[22px] font-bold tabular-nums">18.2 mi</span><span className="text-xs text-muted">Distance</span></div>
            <div><span className="block font-display text-[22px] font-bold tabular-nums">4,550 ft</span><span className="text-xs text-muted">Gain</span></div>
            <div><span className="block font-display text-[22px] font-bold tabular-nums">18 lb 12 oz</span><span className="text-xs text-muted">Pack weight</span></div>
          </div>
        </div>
        <div className="card px-[18px] py-1">
          <div className="flex justify-between gap-3 border-b border-line py-[13px] last:border-0 [&_b]:text-[15px]"><div><b>Day 1 · Colchuck Lake</b><div className="text-xs text-muted">+2,280 ft · camp at 5,580 ft</div></div><span className="whitespace-nowrap text-sm tabular-nums text-muted">4.2 mi</span></div>
          <div className="flex justify-between gap-3 border-b border-line py-[13px] last:border-0 [&_b]:text-[15px]"><div><b>Day 2 · Sprite Lake</b><div className="text-xs text-muted">+2,100 ft · over Aasgard Pass</div></div><span className="whitespace-nowrap text-sm tabular-nums text-muted">4.8 mi</span></div>
          <div className="flex justify-between gap-3 border-b border-line py-[13px] last:border-0 [&_b]:text-[15px]"><div><b>Day 3 · Snow Lakes TH</b><div className="text-xs text-muted">+170 ft · −6,020 ft</div></div><span className="whitespace-nowrap text-sm tabular-nums text-muted">9.2 mi</span></div>
          <div className="flex justify-between gap-3 border-b border-line py-[13px] last:border-0 [&_b]:text-[15px]"><div><b>Permit</b><div className="text-xs text-muted">Core Zone overnight · 2 people</div></div><span className="whitespace-nowrap text-sm tabular-nums text-muted">Attached</span></div>
        </div>
      </div>}


      {view === "gear" && <div key="gear" className="grid min-h-full gap-2.5 p-2.5 sm:gap-3.5 sm:p-3.5 animate-[landing-view_.28s_cubic-bezier(.16,1,.3,1)] sm:grid-cols-2 sm:grid-rows-[auto_1fr]">
        <div className="card px-[18px] py-4">
          <h3 className="text-[17px] font-bold">Base weight</h3>
          <div className="font-display text-[34px] font-bold tabular-nums">12 lb 8 oz</div>
          <div className="mb-3 mt-3.5 flex h-3 gap-0.5 overflow-hidden rounded-full" aria-hidden="true">
            <span style={{ flex: 49.5, background: categoryColor("Clothing") }}></span><span style={{ flex: 44.5, background: categoryColor("Sleep") }}></span><span style={{ flex: 32, background: categoryColor("Shelter") }}></span><span style={{ flex: 28, background: categoryColor("Storage") }}></span><span style={{ flex: 14.5, background: categoryColor("Kitchen") }}></span><span style={{ flex: 12, background: categoryColor("Electronics") }}></span><span style={{ flex: 9, background: categoryColor("Safety") }}></span><span style={{ flex: 6.5, background: categoryColor("Health & Hygiene") }}></span><span style={{ flex: 4, background: categoryColor("Tools & Repair") }}></span>
          </div>
          <div className="grid grid-cols-1 gap-x-4 gap-y-1.5 text-[13px] sm:grid-cols-2 [&>span]:flex [&>span]:min-w-0 [&>span]:items-center [&>span]:gap-2 [&_em]:ml-auto [&_em]:not-italic [&_em]:tabular-nums [&_em]:text-muted">
            <span><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Clothing") }}></i>Clothing<em>49.5 oz</em></span>
            <span><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Sleep") }}></i>Sleep<em>44.5 oz</em></span>
            <span><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Shelter") }}></i>Shelter<em>32.0 oz</em></span>
            <span><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Storage") }}></i>Storage<em>28.0 oz</em></span>
            <span><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Kitchen") }}></i>Kitchen<em>14.5 oz</em></span>
            <span><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Electronics") }}></i>Electronics<em>12.0 oz</em></span>
          </div>
        </div>
        <div className="card px-[18px] py-4">
          <h3 className="text-[17px] font-bold">On this trip</h3>
          <div className="mb-3 mt-3.5 flex h-3 gap-0.5 overflow-hidden rounded-full" aria-hidden="true"><span style={{ flex: 200 }} className="bg-cbase"></span><span style={{ flex: 100 }} className="bg-ccons"></span><span style={{ flex: 36 }} className="bg-cworn"></span></div>
          <div className="flex justify-between gap-3 border-b border-line py-[13px] last:border-0 [&_b]:text-[15px]"><span><Badge tone="base">Base</Badge></span><span className="whitespace-nowrap text-sm tabular-nums text-muted">12 lb 8 oz</span></div>
          <div className="flex justify-between gap-3 border-b border-line py-[13px] last:border-0 [&_b]:text-[15px]"><span><Badge tone="consumable">Consumable</Badge> <span className="text-xs text-muted">food, 1 L water, fuel</span></span><span className="whitespace-nowrap text-sm tabular-nums text-muted">6 lb 4 oz</span></div>
          <div className="flex justify-between gap-3 border-b border-line py-[13px] last:border-0 [&_b]:text-[15px]"><span><Badge tone="worn">Worn</Badge></span><span className="whitespace-nowrap text-sm tabular-nums text-muted">2 lb 4 oz</span></div>
          <div className="flex justify-between gap-3 border-b border-line py-[13px] last:border-0 [&_b]:text-[15px]"><b>Skin-out</b><span className="whitespace-nowrap text-sm tabular-nums font-semibold text-ink">21 lb 0 oz</span></div>
        </div>
        <div className="card col-span-full overflow-hidden">
          <div className="flex items-center gap-2 border-b border-line bg-panel2/60 px-4 py-[9px] font-display text-[15px] font-semibold"><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Shelter") }}></i>Shelter</div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>Durston X-Mid 1</span><Badge tone="base">Base</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">28.9 oz</span></div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>Stakes ×6, guyline</span><Badge tone="base">Base</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">3.1 oz</span></div>
          <div className="flex items-center gap-2 border-b border-line bg-panel2/60 px-4 py-[9px] font-display text-[15px] font-semibold"><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Sleep") }}></i>Sleep</div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>20° quilt</span><Badge tone="base">Base</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">21.4 oz</span></div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>NeoAir XLite, regular</span><Badge tone="base">Base</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">12.5 oz</span></div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>Down booties and sleep layer</span><Badge tone="base">Base</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">10.6 oz</span></div>
          <div className="flex items-center gap-2 border-b border-line bg-panel2/60 px-4 py-[9px] font-display text-[15px] font-semibold"><i className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: categoryColor("Kitchen") }}></i>Kitchen</div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>Toaks 750 pot, lid</span><Badge tone="base">Base</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">3.6 oz</span></div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>BRS-3000T stove, lighter</span><Badge tone="base">Base</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">1.4 oz</span></div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>Bear canister share (BV425)</span><Badge tone="base">Base</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">9.5 oz</span></div>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 border-b border-line px-4 py-2.5 text-sm last:border-0"><span>Fuel, 110 g canister</span><Badge tone="consumable">Consumable</Badge><span className="whitespace-nowrap text-sm tabular-nums text-muted">6.5 oz</span></div>
        </div>
      </div>}


      {view === "food" && <div key="food" className="grid min-h-full gap-2.5 p-2.5 sm:gap-3.5 sm:p-3.5 animate-[landing-view_.28s_cubic-bezier(.16,1,.3,1)] grid-rows-[auto_1fr]">
        <div className="flex flex-wrap items-baseline gap-x-8 gap-y-2.5 rounded-xl border border-line bg-panel2/40 px-[18px] py-3.5">
          <div><span className="block font-display text-[22px] font-bold tabular-nums">7,350 kcal</span><span className="text-xs text-muted">Planned for 3 days</span></div>
          <div><span className="block font-display text-[22px] font-bold tabular-nums">3 lb 10.5 oz</span><span className="text-xs text-muted">Food carried</span></div>
          <div><span className="block font-display text-[22px] font-bold tabular-nums">126 kcal/oz</span><span className="text-xs text-muted">Calorie density</span></div>
        </div>
        <div className="grid content-stretch gap-3.5 desk:grid-cols-3">
          <div className="card px-[18px] py-4">
            <div className="flex flex-wrap items-baseline gap-x-3.5 gap-y-1"><b className="font-display">Day 1</b><span className="text-xs text-muted">17.5 oz</span><span className="ml-auto text-sm tabular-nums">2,220 <span className="text-xs text-muted">/ 2,600 kcal</span></span></div>
            <div className="mt-3 rounded-[10px] border border-line">
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Tortilla, salami, cheddar</span><span>890 kcal</span><span>7.0 oz</span></div>
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Bars and trail mix</span><span>560 kcal</span><span>4.0 oz</span></div>
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Peanut noodles</span><span>770 kcal</span><span>6.5 oz</span></div>
            </div>
          </div>
          <div className="card px-[18px] py-4">
            <div className="flex flex-wrap items-baseline gap-x-3.5 gap-y-1"><b className="font-display">Day 2</b><span className="text-xs text-muted">25.5 oz</span><span className="ml-auto text-sm tabular-nums">3,190 <span className="text-xs text-muted">/ 3,100 kcal</span></span></div>
            <div className="mt-3 rounded-[10px] border border-line">
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Overnight oats</span><span>620 kcal</span><span>5.0 oz</span></div>
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Tortilla, salami, cheddar</span><span>890 kcal</span><span>7.0 oz</span></div>
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Bars and trail mix</span><span>560 kcal</span><span>4.0 oz</span></div>
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Dark chocolate</span><span>300 kcal</span><span>2.0 oz</span></div>
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Red lentil curry</span><span>820 kcal</span><span>7.5 oz</span></div>
            </div>
          </div>
          <div className="card px-[18px] py-4">
            <div className="flex flex-wrap items-baseline gap-x-3.5 gap-y-1"><b className="font-display">Day 3</b><span className="text-xs text-muted">15.5 oz</span><span className="ml-auto text-sm tabular-nums">1,940 <span className="text-xs text-muted">/ 2,400 kcal</span></span></div>
            <div className="mt-3 rounded-[10px] border border-line">
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Overnight oats</span><span>620 kcal</span><span>5.0 oz</span></div>
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Bagel, peanut butter</span><span>760 kcal</span><span>6.5 oz</span></div>
              <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] gap-2.5 border-b border-line px-3 py-[9px] text-sm last:border-0 [&>span+span]:self-center [&>span+span]:text-right [&>span+span]:text-xs [&>span+span]:tabular-nums [&>span+span]:text-muted"><span>Bars and trail mix</span><span>560 kcal</span><span>4.0 oz</span></div>
            </div>
          </div>
        </div>
      </div>}
      </div>
    </section>
  );
}
