"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { Backpack, CalendarDays, Home, Map, Settings, Utensils } from "lucide-react";
import { cn } from "@/lib/util";
import { useUnsaved } from "@/components/unsaved-changes";
import { ContourBand } from "@/components/contour-band";

const NAV = [
  { href: "/basecamp", label: "Basecamp", icon: Home },
  { href: "/gear", label: "Gear", icon: Backpack },
  { href: "/food", label: "Food", icon: Utensils },
  { href: "/trips", label: "Trips", icon: Map },
];

const MOBILE_NAV = [...NAV, { href: "/calendar", label: "Calendar", icon: CalendarDays }];

export function Shell({ children, signedIn }: { children: React.ReactNode; signedIn: boolean }) {
  const path = usePathname();
  const { requestNavigate } = useUnsaved();
  const sharedTrip = path.startsWith("/share/trips/");
  const tripsHref = sharedTrip ? "/trips?section=shared" : "/trips";
  const isActive = (href: string) => path.startsWith(href) || (href === "/trips" && sharedTrip);

  // Remember the last place visited under /trips (list or a specific trip). The
  // print view is a dead-end sub-page, so collapse it to its trip page — coming
  // back to "Trips" should land on the trip, not the print sheet.
  useEffect(() => {
    if (path.startsWith("/trips")) localStorage.setItem("bp_lastTrips", path.replace(/\/print$/, ""));
  }, [path]);

  if (path === "/" || path === "/login" || path === "/privacy" || (path.startsWith("/share/") && !signedIn)) return <>{children}</>;

  // Route every nav click through the unsaved-changes guard. "Trips" resolves to
  // wherever you last were in the trips section.
  const onNav = (href: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    let dest = href;
    if (href === "/trips") {
      if (sharedTrip) dest = tripsHref;
      else {
        const last = typeof window !== "undefined" ? localStorage.getItem("bp_lastTrips") : null;
        if (last && last !== "/trips") dest = last;
      }
    }
    requestNavigate(dest);
  };
  return (
    <div className="min-h-screen pb-[calc(4.5rem+env(safe-area-inset-bottom))] sm:pb-0">
      <header data-noprint className="relative overflow-hidden bg-header text-headerink">
        <ContourBand />
        <div className="relative flex min-h-14 items-center justify-between px-4 sm:hidden">
          <Link href="/basecamp" onClick={onNav("/basecamp")} className="font-display text-xl font-bold text-headerink">
            Strider
          </Link>
          <Link href="/settings" onClick={onNav("/settings")} aria-label="Settings" aria-current={isActive("/settings") ? "page" : undefined} className="grid h-11 w-11 place-items-center rounded-xl -mr-2">
            <Settings size={21} />
          </Link>
        </div>
        <div className="relative mx-auto hidden min-h-16 max-w-[1136px] flex-wrap items-center gap-x-7 gap-y-1 px-6 sm:flex">
          <Link href="/basecamp" onClick={onNav("/basecamp")} className="font-display text-[22px] font-bold">
            Strider
          </Link>
          <nav aria-label="Primary navigation" className="flex flex-wrap gap-1">
            {[...MOBILE_NAV, { href: "/settings", label: "Settings" }].map((item) => (
              <Link key={item.href} href={item.href === "/trips" ? tripsHref : item.href} onClick={onNav(item.href)}
                aria-current={isActive(item.href) ? "page" : undefined}
                className={cn("inline-flex min-h-11 items-center rounded-[10px] px-3 text-[15px]",
                  isActive(item.href) ? "bg-headeractive font-bold text-white" : "font-medium text-headermuted hover:text-headerink")}>
                {item.label}
              </Link>
            ))}
          </nav>
        </div>
      </header>
      {sharedTrip ? children : <main>{children}</main>}
      <nav data-noprint aria-label="Primary navigation"
        className="fixed inset-x-0 bottom-0 z-[2000] grid grid-cols-5 border-t border-line bg-panel px-1 pb-[env(safe-area-inset-bottom)] sm:hidden">
        {MOBILE_NAV.map((item) => {
          const Icon = item.icon;
          return (
            <Link key={item.href} href={item.href === "/trips" ? tripsHref : item.href} onClick={onNav(item.href)}
              aria-current={isActive(item.href) ? "page" : undefined}
              className={cn("flex min-h-[58px] flex-col items-center justify-center gap-[3px] text-xs",
                isActive(item.href) ? "font-bold text-accent" : "font-medium text-muted")}>
              <Icon size={23} />
              <span>{item.label}</span>
            </Link>
          );
        })}
      </nav>
    </div>
  );
}
