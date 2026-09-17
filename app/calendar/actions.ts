"use server";

import { and, eq, isNotNull } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { addDays } from "@/lib/weather";
import { expandEvents, type CalEvent, type Feed } from "@/lib/calendar";

const pad = (n: number) => String(n).padStart(2, "0");
const isoOf = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

export type CalCell = {
  iso: string;
  day: number;
  inMonth: boolean;
  events: CalEvent[];
  tripIds: number[];
  conflict: boolean;
};
export type CalendarData = {
  year: number;
  month: number; // 0-indexed
  label: string;
  cells: CalCell[];
  feeds: { id: number; label: string; color: string }[];
  trips: { id: number; name: string; startDay: string; endDay: string }[];
};

// A 6-week (42-cell) Sunday-start grid for the given month, with iCal events and
// trip date-ranges placed per day and conflicts (trip day that also has an event).
export async function getCalendarData(year: number, month: number): Promise<CalendarData> {
  const userId = await getCurrentUserId();

  const feedRows = await db.select().from(schema.calendarFeed).where(eq(schema.calendarFeed.userId, userId));
  const feeds: Feed[] = feedRows.map((f) => ({ id: f.id, label: f.label, color: f.color, url: f.url }));

  const startOffset = new Date(year, month, 1).getDay();
  const cellsDates = Array.from({ length: 42 }, (_, i) => new Date(year, month, 1 - startOffset + i));
  const from = cellsDates[0];
  const to = new Date(year, month, 1 - startOffset + 41, 23, 59, 59);

  const [events, tripRows] = await Promise.all([
    expandEvents(feeds, from, to),
    db
      .select({ id: schema.trip.id, name: schema.trip.name, startDate: schema.trip.startDate, nights: schema.trip.nights })
      .from(schema.trip)
      .where(and(eq(schema.trip.userId, userId), isNotNull(schema.trip.startDate))),
  ]);

  const gridStart = isoOf(from);
  const gridEnd = isoOf(cellsDates[41]);
  const trips = tripRows
    .map((t) => {
      const startDay = t.startDate!;
      const endDay = addDays(startDay, Math.max(0, t.nights ?? 0));
      return { id: t.id, name: t.name, startDay, endDay };
    })
    .filter((t) => t.endDay >= gridStart && t.startDay <= gridEnd);

  const cells: CalCell[] = cellsDates.map((d) => {
    const iso = isoOf(d);
    const dayEvents = events.filter((e) => e.startDay <= iso && e.endDay >= iso);
    const tripIds = trips.filter((t) => t.startDay <= iso && t.endDay >= iso).map((t) => t.id);
    return {
      iso,
      day: d.getDate(),
      inMonth: d.getMonth() === month,
      events: dayEvents,
      tripIds,
      conflict: tripIds.length > 0 && dayEvents.length > 0,
    };
  });

  return {
    year,
    month,
    label: `${MONTHS[month]} ${year}`,
    cells,
    feeds: feeds.map((f) => ({ id: f.id, label: f.label, color: f.color })),
    trips,
  };
}
