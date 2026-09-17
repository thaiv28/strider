"use client";

import Link from "next/link";
import { useMemo, useRef, useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Card, Button } from "@/components/ui";
import { getCalendarData, type CalendarData } from "@/app/calendar/actions";
import { updateTrip } from "@/app/trips/actions";
import { addDays } from "@/lib/weather";

const WD = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const pad = (n: number) => String(n).padStart(2, "0");
const todayIso = () => {
  const d = new Date();
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};
const daysBetween = (a: string, b: string) =>
  Math.round((Date.parse(b + "T00:00:00Z") - Date.parse(a + "T00:00:00Z")) / 86400000);

type Seg = { id: number; name: string; startCol: number; endCol: number; continuesLeft: boolean; continuesRight: boolean; lane: number };
const LANE_PX = 20; // per trip-bar lane
const DAYNUM_PX = 26; // day-number row height, before the bars start

export function CalendarView({ initial }: { initial: CalendarData }) {
  const [data, setData] = useState(initial);
  const [pending, start] = useTransition();
  const today = useMemo(todayIso, []);
  const router = useRouter();
  const dragged = useRef(false);

  // Reflect the viewed month in the URL (no navigation/refetch) so returning
  // from a trip — or reloading — lands back on the same month.
  const go = (year: number, month: number) =>
    start(async () => {
      setData(await getCalendarData(year, month));
      window.history.replaceState(null, "", `/calendar?y=${year}&m=${month}`);
    });
  const shift = (delta: number) => {
    const m = data.month + delta;
    go(data.year + Math.floor(m / 12), ((m % 12) + 12) % 12);
  };
  const jumpToday = () => {
    const d = new Date();
    go(d.getFullYear(), d.getMonth());
  };

  const [dragId, setDragId] = useState<number | null>(null);
  const [overIso, setOverIso] = useState<string | null>(null);

  // Reschedule a trip by shifting its start to the drop day (duration preserved),
  // optimistically re-placing it on the grid before the server confirms.
  const moveTrip = (tripId: number, dropIso: string) => {
    setData((d) => {
      const t = d.trips.find((x) => x.id === tripId);
      if (!t || t.startDay === dropIso) return d;
      const nights = daysBetween(t.startDay, t.endDay);
      const endIso = addDays(dropIso, nights);
      const trips = d.trips.map((x) => (x.id === tripId ? { ...x, startDay: dropIso, endDay: endIso } : x));
      const cells = d.cells.map((c) => {
        const others = c.tripIds.filter((id) => id !== tripId);
        const tripIds = dropIso <= c.iso && c.iso <= endIso ? [...others, tripId] : others;
        return { ...c, tripIds, conflict: tripIds.length > 0 && c.events.length > 0 };
      });
      return { ...d, trips, cells };
    });
    start(async () => await updateTrip(tripId, { startDate: dropIso }));
  };

  // Split the 42 cells into weeks and, per week, lay out each intersecting trip
  // as a spanning bar assigned to a lane so overlapping trips stack.
  const weeks = useMemo(() => {
    const out: { cells: typeof data.cells; segs: Seg[]; laneCount: number }[] = [];
    for (let w = 0; w < 6; w++) {
      const cells = data.cells.slice(w * 7, w * 7 + 7);
      const weekStart = cells[0].iso, weekEnd = cells[6].iso;
      const segs: Seg[] = [];
      for (const t of data.trips) {
        if (t.endDay < weekStart || t.startDay > weekEnd) continue;
        const sIdx = cells.findIndex((c) => c.iso === t.startDay);
        const eIdx = cells.findIndex((c) => c.iso === t.endDay);
        segs.push({
          id: t.id,
          name: t.name,
          startCol: sIdx === -1 ? 0 : sIdx,
          endCol: eIdx === -1 ? 6 : eIdx,
          continuesLeft: sIdx === -1,
          continuesRight: eIdx === -1,
          lane: 0,
        });
      }
      segs.sort((a, b) => a.startCol - b.startCol || a.id - b.id);
      const laneEnd: number[] = [];
      for (const s of segs) {
        let lane = laneEnd.findIndex((end) => end < s.startCol);
        if (lane === -1) { lane = laneEnd.length; laneEnd.push(s.endCol); } else laneEnd[lane] = s.endCol;
        s.lane = lane;
      }
      out.push({ cells, segs, laneCount: laneEnd.length });
    }
    return out;
  }, [data.cells, data.trips]);

  return (
    <div className={pending ? "opacity-60 transition" : "transition"}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <button onClick={() => shift(-1)} className="rounded-md border bg-panel p-1.5 hover:bg-panel2" aria-label="Previous month"><ChevronLeft size={16} /></button>
          <button onClick={() => shift(1)} className="rounded-md border bg-panel p-1.5 hover:bg-panel2" aria-label="Next month"><ChevronRight size={16} /></button>
          <h2 className="font-display ml-1 text-xl font-bold tracking-tight">{data.label}</h2>
        </div>
        <div className="flex items-center gap-3">
          <Legend feeds={data.feeds} />
          <Button variant="outline" onClick={jumpToday}>Today</Button>
        </div>
      </div>

      {data.feeds.length === 0 && (
        <p className="mt-3 text-sm text-muted">
          No calendars connected yet — add a secret iCal address in{" "}
          <Link href="/settings" className="text-accent hover:underline">Settings</Link>. Trips still show below.
        </p>
      )}

      <Card className="mt-4 overflow-hidden">
        <div className="grid grid-cols-7 border-b bg-panel2/50">
          {WD.map((d) => (
            <div key={d} className="eyebrow px-1 py-1.5 text-center sm:px-2">{d}</div>
          ))}
        </div>
        {weeks.map((week, w) => (
          <div key={w} className="relative border-b last:border-b-0">
            <div className="grid grid-cols-7">
              {week.cells.map((c) => (
                <div
                  key={c.iso}
                  onDragOver={(e) => { if (dragId != null) { e.preventDefault(); setOverIso(c.iso); } }}
                  onDrop={(e) => {
                    e.preventDefault();
                    const id = Number(e.dataTransfer.getData("text/trip"));
                    if (id) moveTrip(id, c.iso);
                    setDragId(null);
                    setOverIso(null);
                  }}
                  className={[
                    "min-h-16 border-r p-1 [&:nth-child(7n)]:border-r-0 sm:min-h-24 sm:p-1.5",
                    c.inMonth ? "" : "bg-panel2/30 text-muted",
                    c.conflict ? "bg-cworn/15" : "",
                    overIso === c.iso ? "ring-2 ring-inset ring-accent" : "",
                  ].join(" ")}
                >
                  <div className="flex items-center justify-between">
                    <span className={`readout text-xs ${c.iso === today ? "flex h-5 w-5 items-center justify-center rounded-full bg-accent font-bold text-accentink" : "text-muted"}`}>{c.day}</span>
                    {c.conflict && <span className="text-[0.65rem] text-cworn" title="Trip overlaps a calendar event">⚠</span>}
                  </div>
                  <div aria-hidden style={{ height: week.laneCount * LANE_PX }} />
                  <div className="space-y-0.5">
                    {c.events.slice(0, 3).map((e, j) => (
                      <div key={j} className="flex items-center gap-1 truncate text-[0.65rem]" title={`${e.time ? e.time + " · " : ""}${e.title} (${e.label})`}>
                        <span className="h-1.5 w-1.5 shrink-0 rounded-full" style={{ background: e.color }} />
                        {e.time && <span className="readout shrink-0 text-muted">{e.time}</span>}
                        <span className="truncate">{e.title}</span>
                      </div>
                    ))}
                    {c.events.length > 3 && <div className="text-[0.6rem] text-muted">+{c.events.length - 3} more</div>}
                  </div>
                </div>
              ))}
            </div>

            <div className="pointer-events-none absolute inset-x-0" style={{ top: DAYNUM_PX }}>
              <div className="grid grid-cols-7" style={{ gridAutoRows: `${LANE_PX}px` }}>
                {week.segs.map((s) => (
                  <div
                    key={s.id}
                    draggable
                    onDragStart={(e) => { dragged.current = true; e.dataTransfer.setData("text/trip", String(s.id)); e.dataTransfer.effectAllowed = "move"; setDragId(s.id); }}
                    onDragEnd={() => { setDragId(null); setOverIso(null); }}
                    onClick={() => { if (dragged.current) { dragged.current = false; return; } router.push(`/trips/${s.id}?from=calendar&y=${data.year}&m=${data.month}`); }}
                    style={{ gridColumnStart: s.startCol + 1, gridColumnEnd: s.endCol + 2, gridRowStart: s.lane + 1 }}
                    className={[
                      "pointer-events-auto mx-0.5 h-[18px] cursor-pointer overflow-hidden truncate bg-accent px-1.5 text-[0.65rem] font-medium leading-[18px] text-accentink active:cursor-grabbing",
                      s.continuesLeft ? "rounded-l-none" : "rounded-l",
                      s.continuesRight ? "rounded-r-none" : "rounded-r",
                    ].join(" ")}
                    title={`${s.name} — click to open · drag to reschedule`}
                  >
                    {s.continuesLeft ? "‹ " : ""}{s.name}
                  </div>
                ))}
              </div>
            </div>
          </div>
        ))}
      </Card>
    </div>
  );
}

function Legend({ feeds }: { feeds: { id: number; label: string; color: string }[] }) {
  return (
    <div className="hidden flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted sm:flex">
      <span className="flex items-center gap-1"><span className="h-2.5 w-2.5 rounded-sm bg-accent" /> Trip</span>
      {feeds.map((f) => (
        <span key={f.id} className="flex items-center gap-1">
          <span className="h-2.5 w-2.5 rounded-full" style={{ background: f.color }} /> {f.label}
        </span>
      ))}
    </div>
  );
}
