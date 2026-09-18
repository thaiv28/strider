import { getCalendarData } from "./actions";
import { CalendarView } from "@/components/calendar/calendar-view";

export const dynamic = "force-dynamic";
export const metadata = { title: "Calendar" };

export default async function CalendarPage({ searchParams }: { searchParams: Promise<{ y?: string; m?: string }> }) {
  const { y, m } = await searchParams;
  const now = new Date();
  const year = y && Number.isFinite(Number(y)) ? Number(y) : now.getFullYear();
  const month = m != null && m !== "" && Number.isFinite(Number(m)) ? Number(m) : now.getMonth();
  const data = await getCalendarData(year, month);

  return (
    <div className="mx-auto max-w-5xl px-4 py-6 sm:px-8 sm:py-10">
      <div className="eyebrow">◇ Planning</div>
      <h1 className="font-display mt-1 mb-6 text-3xl font-bold tracking-tight">Calendar</h1>
      <CalendarView initial={data} />
    </div>
  );
}
