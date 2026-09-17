import ical from "node-ical";

type CalData = Awaited<ReturnType<typeof ical.async.fromURL>>;

// Read-only iCal feed parsing for the planning calendar. Events are normalized
// to inclusive [startDay, endDay] date strings for month-grid placement, which
// sidesteps most timezone/all-day off-by-one issues.

export type Feed = { id: number; label: string; color: string; url: string };
export type CalEvent = {
  feedId: number;
  label: string;
  color: string;
  title: string;
  startDay: string; // YYYY-MM-DD (inclusive)
  endDay: string; // YYYY-MM-DD (inclusive)
  allDay: boolean;
  time: string | null;
};

const TTL = 5 * 60 * 1000;
const cache = new Map<string, { at: number; data: CalData }>();

async function load(url: string): Promise<CalData | null> {
  const hit = cache.get(url);
  const now = Date.now();
  if (hit && now - hit.at < TTL) return hit.data;
  try {
    const data = await ical.async.fromURL(url);
    cache.set(url, { at: now, data });
    return data;
  } catch {
    return null;
  }
}

const pad = (n: number) => String(n).padStart(2, "0");
const localDay = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const utcDay = (d: Date) => `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}`;
function timeLabel(d: Date): string {
  let h = d.getHours();
  const m = d.getMinutes();
  const ap = h < 12 ? "am" : "pm";
  h = h % 12 || 12;
  return m ? `${h}:${pad(m)} ${ap}` : `${h} ${ap}`;
}

/* eslint-disable @typescript-eslint/no-explicit-any */
function isAllDay(ev: any): boolean {
  return ev?.datetype === "date" || ev?.start?.dateOnly === true;
}

function normalize(ev: any, feed: Feed): CalEvent {
  const allDay = isAllDay(ev);
  const start: Date = ev.start;
  const end: Date = ev.end ?? start;
  const startDay = allDay ? utcDay(start) : localDay(start);
  // iCal DTEND is exclusive for all-day spans; step back one ms for the last day.
  const endDay = allDay ? utcDay(new Date(end.getTime() - 1)) : localDay(end);
  return {
    feedId: feed.id,
    label: feed.label,
    color: feed.color,
    title: (ev.summary || "(untitled)").toString(),
    startDay,
    endDay: endDay < startDay ? startDay : endDay,
    allDay,
    time: allDay ? null : timeLabel(start),
  };
}

// Expand one feed's VEVENTs (including simple recurrences) overlapping [from, to].
function expandFeed(data: CalData, feed: Feed, from: Date, to: Date): CalEvent[] {
  const fromISO = localDay(from);
  const toISO = localDay(to);
  const overlaps = (e: CalEvent) => e.endDay >= fromISO && e.startDay <= toISO;
  const out: CalEvent[] = [];

  for (const key of Object.keys(data)) {
    const ev: any = (data as any)[key];
    if (ev?.type !== "VEVENT") continue;

    if (ev.rrule) {
      const durMs = (ev.end?.getTime?.() ?? ev.start.getTime()) - ev.start.getTime();
      const exdates = new Set(Object.keys(ev.exdate ?? {}).map((k) => utcDay(new Date(ev.exdate[k]))));
      const occs: Date[] = ev.rrule.between(from, to, true);
      for (const occ of occs) {
        const occKey = utcDay(occ);
        if (exdates.has(occKey)) continue;
        const override = ev.recurrences?.[occKey] ?? ev.recurrences?.[localDay(occ)];
        const e = override
          ? normalize(override, feed)
          : normalize({ ...ev, rrule: undefined, start: occ, end: new Date(occ.getTime() + durMs) }, feed);
        if (overlaps(e)) out.push(e);
      }
    } else {
      const e = normalize(ev, feed);
      if (overlaps(e)) out.push(e);
    }
  }
  return out;
}

export async function expandEvents(feeds: Feed[], from: Date, to: Date): Promise<CalEvent[]> {
  const per = await Promise.all(
    feeds.map(async (f) => {
      const data = await load(f.url);
      return data ? expandFeed(data, f, from, to) : [];
    }),
  );
  return per.flat();
}
