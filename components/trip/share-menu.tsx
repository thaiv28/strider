"use client";

import { useEffect, useRef, useState, useTransition } from "react";
import { Check, ExternalLink, FileText, Link2, Printer, Share2, Unlink } from "lucide-react";
import { createPortal } from "react-dom";
import { Sheet } from "@/components/sheet";
import { Button } from "@/components/ui";
import { createTripShareLink, revokeTripShareLink, createTripEditLink, revokeTripEditLink } from "@/app/trips/actions";

export function ShareMenu({
  icon = false,
  tripId,
  initialToken,
  initialEditToken,
  onGenerateReport,
}: {
  icon?: boolean;
  tripId: number;
  initialToken: string | null;
  initialEditToken: string | null;
  onGenerateReport: () => void;
}) {
  const [phone, setPhone] = useState(false);
  useEffect(() => {
    const query = window.matchMedia("(max-width: 639px)");
    const update = () => setPhone(query.matches);
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  const [open, setOpen] = useState(false);
  const [token, setToken] = useState(initialToken);
  const [editToken, setEditToken] = useState(initialEditToken);
  const [copied, setCopied] = useState(false);
  const [confirmRevoke, setConfirmRevoke] = useState(false);
  const [confirmEditRevoke, setConfirmEditRevoke] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [pending, start] = useTransition();
  const popup = useRef<HTMLDivElement>(null);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node) && !popup.current?.contains(event.target as Node)) setOpen(false);
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

  const menu = <div role="menu" aria-label="Trip sharing" className="text-ink">
    <a href={`/trips/${tripId}/print`} target="_blank" rel="noreferrer" role="menuitem" className="flex min-h-11 items-center gap-3 rounded-xl px-3 py-2 text-sm hover:bg-panel2">
      <Printer size={18} className="shrink-0 text-muted" />
      <span><span className="block font-semibold">Print trip</span><span className="block text-xs text-muted">Open the printable trip sheet</span></span>
    </a>
    <button type="button" role="menuitem" onClick={() => { setOpen(false); onGenerateReport(); }} className="flex min-h-11 w-full items-center gap-3 rounded-xl px-3 py-2 text-left text-sm hover:bg-panel2">
      <FileText size={18} className="shrink-0 text-muted" />
      <span><span className="block font-semibold">Generate pre-trip report</span><span className="block text-xs text-muted">Choose sections and export</span></span>
    </button>
    <div className="mt-3 rounded-xl bg-panel2 p-3">
      <div className="font-semibold">View-only link</div>
      <p className="mt-1 text-xs text-muted">Live trip details, no private notes</p>
      <div className="mt-2 flex gap-2">
        <Button role="menuitem" className="flex-1" aria-label={copied ? "Link copied" : token ? "Copy view-only link" : "Create view-only link"} onClick={copyLink} disabled={pending}>
          {copied ? <Check size={16} /> : <Link2 size={16} />}{copied ? "Copied" : token ? "Copy link" : "Create link"}
        </Button>
        {token && <Button role="menuitem" variant="danger" aria-label={confirmRevoke ? "Confirm revoke" : "Revoke view-only link"} onClick={revoke} disabled={pending}><Unlink size={16} />{confirmRevoke ? "Confirm" : "Revoke"}</Button>}
      </div>
      {token && <a href={`/share/trips/${token}`} target="_blank" rel="noreferrer" role="menuitem" className="mt-1 flex min-h-11 items-center gap-2 text-xs text-muted hover:text-ink"><ExternalLink size={14} />Open view-only page</a>}
    </div>
    <div className="mt-3 rounded-xl bg-panel2 p-3">
      <div className="font-semibold">Edit link</div>
      <p className="mt-1 text-xs text-muted">Sign-in required · private notes excluded</p>
      <div className="mt-2 flex gap-2">
        <Button role="menuitem" className="flex-1" aria-label={editToken ? "Copy edit link" : "Create edit link"} onClick={copyEditLink} disabled={pending}><Link2 size={16} />{editToken ? "Copy link" : "Create link"}</Button>
        {editToken && <Button role="menuitem" variant="danger" aria-label={confirmEditRevoke ? "Confirm revoke edit link" : "Revoke edit link"} onClick={revokeEdit} disabled={pending}><Unlink size={16} />{confirmEditRevoke ? "Confirm" : "Revoke"}</Button>}
      </div>
      {editToken && <a href={`/share/trips/${editToken}`} target="_blank" rel="noreferrer" role="menuitem" className="mt-1 flex min-h-11 items-center gap-2 text-xs text-muted hover:text-ink"><ExternalLink size={14} />Open edit link</a>}
    </div>
    {message && <div role="status" className="mt-3 border-t px-3 py-2 text-xs leading-relaxed text-muted">{message}</div>}
  </div>;

  return (
    <div ref={root} className="relative">
      <Button variant={icon ? "headerOutline" : "header"} className={icon ? "w-11 px-0" : undefined} aria-label="Share" onClick={() => setOpen((v) => !v)} aria-haspopup="menu" aria-expanded={open}>
        {icon && <Share2 size={18} />}{!icon && "Share"}
      </Button>
      {phone ? <Sheet open={open} onClose={() => setOpen(false)} title="Share trip">{menu}</Sheet> : open && createPortal(
        <div ref={popup} style={{ top: (root.current?.getBoundingClientRect().bottom ?? 0) + 8, right: Math.max(16, window.innerWidth - (root.current?.getBoundingClientRect().right ?? window.innerWidth)) }} className="fixed z-[2200] w-80 max-w-[calc(100vw-2rem)] rounded-2xl border bg-panel p-3 shadow-xl" onKeyDown={(event) => { if (event.key === "Escape") setOpen(false); }}>
          <h2 className="mb-2 px-3 font-display text-xl font-bold">Share trip</h2>
          {menu}
        </div>, document.body
      )}
    </div>
  );
}
