"use client";

import { useEffect, useId, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { IconButton } from "@/components/ui";

export function Sheet({ open, onClose, title, children }: { open: boolean; onClose: () => void; title: string; children: ReactNode }) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  }, [open]);

  return (
    <dialog ref={dialogRef} className="sheet" aria-labelledby={titleId}
      onCancel={(event) => { event.preventDefault(); onClose(); }}
      onClick={(event) => { if (event.target === dialogRef.current) onClose(); }}>
      <div className="sheet-handle" aria-hidden="true" />
      <div className="flex items-center justify-between gap-3">
        <h2 id={titleId} className="font-display text-[26px] font-bold leading-tight">{title}</h2>
        <IconButton aria-label="Close" onClick={onClose}><X /></IconButton>
      </div>
      {open && children}
    </dialog>
  );
}
