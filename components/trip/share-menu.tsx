"use client";

import { useEffect, useRef, useState, useTransition } from "react";
import { Check, ExternalLink, FileText, Link2, Printer, Share2, Unlink } from "lucide-react";
import { Button } from "@/components/ui";
import { createTripShareLink, revokeTripShareLink, createTripEditLink, revokeTripEditLink } from "@/app/trips/actions";

export function ShareMenu({
  tripId,
  initialToken,
  initialEditToken,
  onGenerateReport,
}: {
  tripId: number;
  initialToken: string | null;
  initialEditToken: string | null;
  onGenerateReport: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [token, setToken] = useState(initialToken);
  const [editToken, setEditToken] = useState(initialEditToken);
  const [copied, setCopied] = useState(false);
  const [confirmRevoke, setConfirmRevoke] = useState(false);
  const [confirmEditRevoke, setConfirmEditRevoke] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [pending, start] = useTransition();
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, []);

  const copyLink = () =>
    start(async () => {
      try {
        const next = token ?? (await createTripShareLink(tripId));
        setToken(next);
        const url = `${window.location.origin}/share/trips/${next}`;
        if (navigator.clipboard?.writeText) await navigator.clipboard.writeText(url);
        setCopied(true);
        setMessage("View-only link copied.");
        window.setTimeout(() => setCopied(false), 1800);
      } catch {
        setMessage("The link is active, but it could not be copied. Open it below and copy the address.");
      }
    });

  const revoke = () => {
    if (!confirmRevoke) {
      setConfirmRevoke(true);
      setMessage("Select revoke again to confirm.");
      return;
    }
    start(async () => {
      try {
        await revokeTripShareLink(tripId);
        setToken(null);
        setCopied(false);
        setConfirmRevoke(false);
        setMessage("View-only link revoked.");
      } catch {
        setMessage("Could not revoke the link. Try again.");
      }
    });
  };

  const copyEditLink = () => start(async () => {
    try {
      const next = editToken ?? await createTripEditLink(tripId);
      setEditToken(next);
      await navigator.clipboard.writeText(`${window.location.origin}/share/trips/${next}`);
      setMessage("Edit link copied. Anyone signed in with this link can edit the trip.");
    } catch {
      setMessage("The edit link is active. Open it below and copy the address.");
    }
  });

  const revokeEdit = () => {
    if (!confirmEditRevoke) { setConfirmEditRevoke(true); setMessage("Select revoke again to confirm."); return; }
    start(async () => {
      try {
        await revokeTripEditLink(tripId);
        setEditToken(null);
        setConfirmEditRevoke(false);
        setMessage("Edit access revoked.");
      } catch { setMessage("Could not revoke the edit link. Try again."); }
    });
  };

  return (
    <div ref={root} className="relative">
      <Button variant="outline" onClick={() => setOpen((v) => !v)} aria-haspopup="menu" aria-expanded={open}>
        <Share2 size={16} /> Share
      </Button>
      {open && (
        <div role="menu" className="absolute right-0 z-[1200] mt-2 w-64 overflow-hidden rounded-lg border bg-panel p-1.5 shadow-xl">
          <a href={`/trips/${tripId}/print`} target="_blank" rel="noreferrer" role="menuitem" className="flex min-h-11 items-center gap-3 rounded-md px-3 py-2 text-sm hover:bg-panel2">
            <Printer size={16} className="text-muted" />
            <span><span className="block font-medium">Print trip</span><span className="block text-xs text-muted">Open the printable trip sheet</span></span>
          </a>
          <button type="button" role="menuitem" onClick={() => { setOpen(false); onGenerateReport(); }} className="flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-panel2">
            <FileText size={16} className="text-muted" />
            <span><span className="block font-medium">Generate pre-trip report</span><span className="block text-xs text-muted">Choose sections and export</span></span>
          </button>
          <button type="button" role="menuitem" onClick={copyLink} disabled={pending} className="flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-panel2 disabled:opacity-50">
            {copied ? <Check size={16} className="text-accent" /> : <Link2 size={16} className="text-muted" />}
            <span><span className="block font-medium">{copied ? "Link copied" : token ? "Copy view-only link" : "Create view-only link"}</span><span className="block text-xs text-muted">Live trip details, no private notes</span></span>
          </button>
          {token && (
            <>
              <a href={`/share/trips/${token}`} target="_blank" rel="noreferrer" role="menuitem" className="flex min-h-11 items-center gap-3 rounded-md px-3 py-2 text-sm text-muted hover:bg-panel2 hover:text-ink">
                <ExternalLink size={16} /> Open view-only page
              </a>
              <button type="button" role="menuitem" onClick={revoke} disabled={pending} className="flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm text-muted hover:bg-panel2 hover:text-ink disabled:opacity-50">
                <Unlink size={16} /> {confirmRevoke ? "Confirm revoke" : "Revoke view-only link"}
              </button>
            </>
          )}
          <div className="mt-1 border-t pt-1">
            <button type="button" role="menuitem" onClick={copyEditLink} disabled={pending} className="flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-panel2 disabled:opacity-50">
              <Link2 size={16} className="text-muted" />
              <span><span className="block font-medium">{editToken ? "Copy edit link" : "Create edit link"}</span><span className="block text-xs text-muted">Sign-in required · private notes excluded</span></span>
            </button>
            {editToken && <>
              <a href={`/share/trips/${editToken}`} target="_blank" rel="noreferrer" role="menuitem" className="flex min-h-11 items-center gap-3 rounded-md px-3 py-2 text-sm text-muted hover:bg-panel2 hover:text-ink"><ExternalLink size={16} /> Open edit link</a>
              <button type="button" role="menuitem" onClick={revokeEdit} disabled={pending} className="flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm text-muted hover:bg-panel2 hover:text-ink disabled:opacity-50"><Unlink size={16} /> {confirmEditRevoke ? "Confirm revoke edit link" : "Revoke edit link"}</button>
            </>}
          </div>
          {message && <div role="status" className="border-t px-3 py-2 text-xs leading-relaxed text-muted">{message}</div>}
        </div>
      )}
    </div>
  );
}
