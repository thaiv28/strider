"use client";

import { useRef, useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui";
import { importTrip } from "@/app/trips/actions";
import { Sheet } from "@/components/sheet";
import { PLAN_PROMPT } from "@/lib/plan-prompt";

export function PlanWithAI() {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [desc, setDesc] = useState("");
  const [copied, setCopied] = useState(false);
  const [paste, setPaste] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [importing, start] = useTransition();
  const fileRef = useRef<HTMLInputElement>(null);

  const fullPrompt = () => `Trip I'm planning:\n${desc.trim() || "(describe your trip here)"}\n\n${PLAN_PROMPT}`;
  const copy = async () => {
    await navigator.clipboard.writeText(fullPrompt());
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
  };

  const create = (json: string) => {
    setError(null);
    start(async () => {
      const res = await importTrip(json);
      if (res.error) setError(res.error);
      else if (res.tripId) router.push(`/trips/${res.tripId}`);
    });
  };
  const onFile = async (f: File | undefined) => {
    if (f) create(await f.text());
  };

  return (
    <>
      <Button variant="headerOutline" onClick={() => setOpen(true)}>Plan with AI</Button>

      <Sheet open={open} onClose={() => !importing && setOpen(false)} title="Plan with AI">
      <p className="mt-2 text-sm text-muted">
        Describe your trip, copy the prompt into a web-capable AI agent, then paste the JSON it returns back here to build the trip.
      </p>

      <div className="mt-4">
        <label htmlFor="ai-trip-description" className="eyebrow">1 · Your trip</label>
        <textarea
          id="ai-trip-description"
          value={desc}
          onChange={(e) => setDesc(e.target.value)}
          rows={4}
          placeholder="e.g. Enchanted Valley in the Olympics, ~3 nights mid-September, 2 people, moderate pace, want a river valley with established camps"
          className="mt-2 w-full resize-y rounded-md border bg-panel2 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/25"
        />
        <div className="mt-2">
          <Button onClick={copy}>{copied ? "Copied!" : "Copy planning prompt"}</Button>
        </div>
      </div>

      <div className="mt-5 border-t pt-4">
        <label htmlFor="ai-trip-result" className="eyebrow">2 · Import the result</label>
        <textarea
          id="ai-trip-result"
          value={paste}
          onChange={(e) => setPaste(e.target.value)}
          rows={5}
          placeholder="Paste the JSON the agent returned…"
          className="readout mt-2 w-full resize-y rounded-md border bg-panel2 px-3 py-2 text-xs outline-none focus:ring-2 focus:ring-accent/25"
        />
        {error && <div className="mt-2 text-sm text-signal">{error}</div>}
        <div className="mt-2 flex items-center gap-2">
          <Button disabled={importing || !paste.trim()} onClick={() => create(paste)}>
            {importing ? "Building…" : "Create trip"}
          </Button>
          <input ref={fileRef} type="file" accept=".json,application/json" className="hidden" onChange={(e) => onFile(e.target.files?.[0])} />
          <Button variant="outline" disabled={importing} onClick={() => fileRef.current?.click()}>Upload .json</Button>
        </div>
      </div>
      </Sheet>
    </>
  );
}
