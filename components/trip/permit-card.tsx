"use client";

import { useRef, useState, useTransition } from "react";
import { FileText, ImageIcon, Upload, Trash2 } from "lucide-react";
import { Card, Button } from "@/components/ui";
import { uploadPermit, deletePermit } from "@/app/trips/actions";

export type PermitMeta = { filename: string; mimeType: string; sizeBytes: number } | null;

const kb = (n: number) => (n < 1024 * 1024 ? `${Math.round(n / 1024)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`);

export function PermitCard({ tripId, permit }: { tripId: number; permit: PermitMeta }) {
  const [pending, start] = useTransition();
  const [err, setErr] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const isPdf = permit?.mimeType === "application/pdf";

  const onFile = (file: File) => {
    setErr(null);
    const form = new FormData();
    form.set("file", file);
    start(async () => {
      const r = await uploadPermit(tripId, form);
      if (!r.ok) setErr(r.error ?? "Upload failed.");
    });
  };

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between">
        <div className="eyebrow">Permit document</div>
        {pending && <span className="text-xs text-muted">saving…</span>}
      </div>

      {permit ? (
        <div className="mt-2 flex items-center gap-3">
          <span className="text-muted">{isPdf ? <FileText size={18} /> : <ImageIcon size={18} />}</span>
          <a href={`/trips/${tripId}/permit`} target="_blank" rel="noreferrer" className="min-w-0 flex-1 truncate text-sm text-accent hover:underline">
            {permit.filename}
          </a>
          <span className="readout shrink-0 text-xs text-muted">{kb(permit.sizeBytes)}</span>
          <Button variant="ghost" onClick={() => inputRef.current?.click()} disabled={pending}>Replace</Button>
          <Button
            variant="ghost"
            aria-label="Remove permit"
            title="Remove permit"
            onClick={() => confirm("Remove the stored permit?") && start(async () => await deletePermit(tripId))}
            disabled={pending}
          >
            <Trash2 size={16} />
          </Button>
        </div>
      ) : (
        <button
          onClick={() => inputRef.current?.click()}
          disabled={pending}
          className="mt-2 flex w-full items-center justify-center gap-2 rounded-md border border-dashed py-4 text-sm text-muted transition hover:border-accent hover:text-ink"
        >
          <Upload size={16} /> Upload permit (PDF or image)
        </button>
      )}

      {err && <p className="mt-2 text-xs text-red-500">{err}</p>}

      <input
        ref={inputRef}
        type="file"
        accept="application/pdf,image/*"
        className="hidden"
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) onFile(f);
          e.target.value = "";
        }}
      />
    </Card>
  );
}
