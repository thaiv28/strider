"use client";

import { useState, useTransition } from "react";
import { Card, Button, Input, Field } from "@/components/ui";
import { addCalendarFeed, updateCalendarFeed, deleteCalendarFeed } from "@/app/settings/actions";

export type FeedRow = { id: number; label: string; color: string; url: string };

export function CalendarFeeds({ feeds }: { feeds: FeedRow[] }) {
  const [, start] = useTransition();
  const [label, setLabel] = useState("");
  const [color, setColor] = useState("#1f7a70");
  const [url, setUrl] = useState("");

  const add = () => {
    if (!url.trim()) return;
    start(async () => {
      await addCalendarFeed({ label, color, url });
      setLabel("");
      setUrl("");
      setColor("#1f7a70");
    });
  };

  return (
    <Card className="min-w-0 p-4 sm:p-5">
      <h2 className="text-[17px] font-bold">Calendars</h2>
      <p className="mt-2 text-sm text-muted">
        Read-only feeds shown on the planning calendar. In Google Calendar → Settings and sharing → Integrate calendar, copy the
        <span className="readout"> Secret address in iCal format</span> (ends in <span className="readout">/basic.ics</span>). Treat these URLs as private.
      </p>

      <div className="mt-4 space-y-2">
        {feeds.length === 0 && <div className="text-sm text-muted">No calendars yet.</div>}
        {feeds.map((f) => (
          <FeedItem key={f.id} feed={f} />
        ))}
      </div>

      <div className="mt-4 grid grid-cols-[minmax(0,1fr)_3rem] items-end gap-3 border-t border-divider pt-4">
        <Field label="Label"><Input placeholder="Personal" value={label} onChange={(e) => setLabel(e.target.value)} /></Field>
        <Field label="Color"><Input type="color" value={color} onChange={(e) => setColor(e.target.value)} className="cursor-pointer px-1" /></Field>
        <Field label="URL" className="col-span-2"><Input type="url" placeholder="https://calendar.google.com/…/basic.ics" value={url} onChange={(e) => setUrl(e.target.value)} /></Field>
        <Button onClick={add} disabled={!url.trim()} className="col-span-2 justify-self-end">Add</Button>
      </div>
    </Card>
  );
}

function FeedItem({ feed }: { feed: FeedRow }) {
  const [, start] = useTransition();
  const [label, setLabel] = useState(feed.label);
  const [color, setColor] = useState(feed.color);
  const [url, setUrl] = useState(feed.url);
  const dirty = label !== feed.label || color !== feed.color || url !== feed.url;

  return (
    <div className="border-b border-divider py-2 last:border-b-0">
      <div className="flex items-start gap-2">
        <details className="min-w-0 flex-1">
          <summary className="flex min-h-11 cursor-pointer list-none items-center gap-3 rounded-lg font-semibold focus-visible:outline-2 focus-visible:outline-accent">
            <span className="h-3 w-3 shrink-0 rounded-full" style={{ background: color }} />
            <span className="min-w-0 break-words">{label || "Calendar"}</span>
            <span className="ml-auto text-xs font-normal text-muted">Edit</span>
          </summary>
          <div className="grid grid-cols-[minmax(0,1fr)_3rem] gap-3 pb-3 pt-2">
            <Field label="Label"><Input value={label} onChange={(e) => setLabel(e.target.value)} /></Field>
            <Field label="Color"><Input type="color" value={color} onChange={(e) => setColor(e.target.value)} className="cursor-pointer px-1" /></Field>
            <Field label="URL" className="col-span-2"><Input value={url} onChange={(e) => setUrl(e.target.value)} /></Field>
            <Button variant="outline" disabled={!dirty} className="col-span-2 justify-self-end" onClick={() => start(async () => await updateCalendarFeed(feed.id, { label, color, url }))}>Save</Button>
          </div>
        </details>
        <Button variant="danger" className="px-2" aria-label={`Remove ${feed.label}`} onClick={() => confirm(`Remove "${feed.label}"?`) && start(async () => await deleteCalendarFeed(feed.id))}>Remove</Button>
      </div>
    </div>
  );
}
