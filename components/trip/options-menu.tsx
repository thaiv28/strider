"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";
import { MoreHorizontal } from "lucide-react";
import { IconButton } from "@/components/ui";
import { Sheet } from "@/components/sheet";

export function OptionsMenu({ label, header = false, children }: { label: string; header?: boolean; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [phone, setPhone] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const query = matchMedia("(max-width: 639px)");
    const sync = () => setPhone(query.matches);
    sync(); query.addEventListener("change", sync);
    const close = (event: PointerEvent) => { if (!root.current?.contains(event.target as Node)) setOpen(false); };
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape") setOpen(false); };
    document.addEventListener("pointerdown", close); document.addEventListener("keydown", escape);
    return () => { query.removeEventListener("change", sync); document.removeEventListener("pointerdown", close); document.removeEventListener("keydown", escape); };
  }, []);
  const menu = <div role="menu" aria-label={label} className="text-ink [&>button]:flex [&>button]:min-h-11 [&>button]:w-full [&>button]:items-center [&>button]:rounded-lg [&>button]:px-3 [&>button]:text-left [&>button]:text-sm [&>button:hover]:bg-panel2" onClick={() => setOpen(false)}>{children}</div>;
  return <div ref={root} className="relative">
    <IconButton aria-label={label} aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen(!open)} className={header ? "border border-headerborder !text-headerink hover:!bg-headeractive" : "!bg-white !text-[#18261f] shadow"}><MoreHorizontal size={20} /></IconButton>
    {phone ? <Sheet open={open} onClose={() => setOpen(false)} title={label}>{menu}</Sheet> : open && <div className="absolute right-0 top-full z-[1200] mt-2 w-64 rounded-xl border bg-panel p-1.5 shadow-lg">{menu}</div>}
  </div>;
}
