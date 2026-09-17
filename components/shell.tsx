"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { CalendarDays, Settings } from "lucide-react";
import { cn } from "@/lib/util";
import { useUnsaved } from "@/components/unsaved-changes";

const NAV = [
  { href: "/", label: "Basecamp" },
  { href: "/gear", label: "Gear" },
  { href: "/food", label: "Food" },
  { href: "/trips", label: "Trips" },
];

export function Shell({ children }: { children: React.ReactNode }) {
  const path = usePathname();
  const { requestNavigate } = useUnsaved();
  const isActive = (href: string) => (href === "/" ? path === "/" : path.startsWith(href));

  // Remember the last place visited under /trips (list or a specific trip). The
  // print view is a dead-end sub-page, so collapse it to its trip page — coming
  // back to "Trips" should land on the trip, not the print sheet.
  useEffect(() => {
    if (path.startsWith("/trips")) localStorage.setItem("bp_lastTrips", path.replace(/\/print$/, ""));
  }, [path]);

  // Route every nav click through the unsaved-changes guard. "Trips" resolves to
  // wherever you last were in the trips section.
  const onNav = (href: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    let dest = href;
    if (href === "/trips") {
      const last = typeof window !== "undefined" ? localStorage.getItem("bp_lastTrips") : null;
      if (last && last !== "/trips") dest = last;
    }
    requestNavigate(dest);
  };
  return (
    <div className="min-h-screen">
      <header data-noprint className="border-b-2 border-double bg-panel">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-3 px-4 py-3 sm:px-8">
          <Link href="/" onClick={onNav("/")} className="shrink-0 font-display text-lg font-bold tracking-tight leading-none sm:text-xl">
            STRIDER
          </Link>
          <nav className="flex min-w-0 items-center gap-2.5 overflow-x-auto [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden sm:gap-6 sm:overflow-visible">
            {NAV.map((l) => (
              <Link
                key={l.href}
                href={l.href}
                onClick={onNav(l.href)}
                className={cn(
                  "shrink-0 font-display text-xs font-semibold tracking-wide uppercase transition sm:text-sm",
                  isActive(l.href) ? "text-accent" : "text-muted hover:text-ink",
                )}
              >
                {l.label}
              </Link>
            ))}
          </nav>
          <div className="flex shrink-0 items-center gap-4">
            <Link
              href="/calendar"
              onClick={onNav("/calendar")}
              aria-label="Calendar"
              className={cn("transition", isActive("/calendar") ? "text-accent" : "text-muted hover:text-ink")}
            >
              <CalendarDays size={18} />
            </Link>
            <Link
              href="/settings"
              onClick={onNav("/settings")}
              aria-label="Settings"
              className={cn("transition", isActive("/settings") ? "text-accent" : "text-muted hover:text-ink")}
            >
              <Settings size={18} />
            </Link>
          </div>
        </div>
      </header>
      <main>{children}</main>
    </div>
  );
}
