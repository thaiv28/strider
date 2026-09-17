"use client";

import { useState, useTransition } from "react";
import { Card, Button, Input } from "@/components/ui";
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
    <Card className="p-5">
      <div className="eyebrow">Calendars</div>
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

      <div className="mt-4 grid grid-cols-[1fr_2.5rem_2fr_auto] items-center gap-2 border-t pt-4">
        <Input placeholder="Label" value={label} onChange={(e) => setLabel(e.target.value)} />
        <input type="color" value={color} onChange={(e) => setColor(e.target.value)} className="h-9 w-full cursor-pointer rounded-md border bg-panel2" aria-label="Color" />
        <Input placeholder="https://calendar.google.com/…/basic.ics" value={url} onChange={(e) => setUrl(e.target.value)} />
        <Button onClick={add} disabled={!url.trim()}>Add</Button>
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
    <div className="grid grid-cols-[1fr_2.5rem_2fr_auto_auto] items-center gap-2">
      <Input value={label} onChange={(e) => setLabel(e.target.value)} />
      <input type="color" value={color} onChange={(e) => setColor(e.target.value)} className="h-9 w-full cursor-pointer rounded-md border bg-panel2" aria-label="Color" />
      <Input value={url} onChange={(e) => setUrl(e.target.value)} className="readout text-xs" />
      <Button variant="outline" disabled={!dirty} onClick={() => start(async () => await updateCalendarFeed(feed.id, { label, color, url }))}>Save</Button>
      <button
        onClick={() => confirm(`Remove "${feed.label}"?`) && start(async () => await deleteCalendarFeed(feed.id))}
        className="rounded p-1.5 text-muted hover:bg-accent/10 hover:text-accent"
        aria-label={`Remove ${feed.label}`}
      >
        ✕
      </button>
    </div>
  );
}
