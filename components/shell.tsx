"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { Backpack, CalendarDays, Home, LogOut, Map, Settings, Utensils } from "lucide-react";
import { cn } from "@/lib/util";
import { useUnsaved } from "@/components/unsaved-changes";
import { logout } from "@/app/login/actions";

const NAV = [
  { href: "/", label: "Basecamp", icon: Home },
  { href: "/gear", label: "Gear", icon: Backpack },
  { href: "/food", label: "Food", icon: Utensils },
  { href: "/trips", label: "Trips", icon: Map },
];

const MOBILE_NAV = [...NAV, { href: "/calendar", label: "Calendar", icon: CalendarDays }];

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

  if (path === "/login") return <>{children}</>;

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
    <div className="min-h-screen pb-[calc(4.5rem+env(safe-area-inset-bottom))] md:pb-0">
      <header data-noprint className="sticky top-0 z-50 border-b-2 border-double bg-panel/95 backdrop-blur md:static md:bg-panel">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-3 px-4 py-3 md:hidden">
          <Link href="/" onClick={onNav("/")} className="font-display text-xl font-bold tracking-tight leading-none">
            STRIDER
          </Link>
          <div className="flex items-center gap-1">
            <Link href="/settings" onClick={onNav("/settings")} aria-label="Settings" className="grid h-11 w-11 place-items-center rounded-md text-muted hover:bg-panel2 hover:text-ink">
              <Settings size={20} />
            </Link>
            <form action={logout}>
              <button type="submit" aria-label="Log out" className="grid h-11 w-11 place-items-center rounded-md text-muted hover:bg-panel2 hover:text-ink">
                <LogOut size={20} />
              </button>
            </form>
          </div>
        </div>
        <div className="mx-auto hidden max-w-6xl items-center justify-between gap-3 px-8 py-3 md:flex">
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
            <form action={logout}>
              <button type="submit" aria-label="Log out" className="text-muted transition hover:text-ink">
                <LogOut size={18} />
              </button>
            </form>
          </div>
        </div>
      </header>
      <main>{children}</main>
      <nav
        data-noprint
        aria-label="Primary navigation"
        className="fixed inset-x-0 bottom-0 z-[2000] grid grid-cols-5 border-t bg-panel/95 px-1 pb-[env(safe-area-inset-bottom)] shadow-[0_-8px_24px_rgba(0,0,0,0.08)] backdrop-blur md:hidden"
      >
        {MOBILE_NAV.map((item) => {
          const Icon = item.icon;
          return (
            <Link
              key={item.href}
              href={item.href}
              onClick={onNav(item.href)}
              aria-current={isActive(item.href) ? "page" : undefined}
              className={cn(
                "flex min-h-16 flex-col items-center justify-center gap-1 rounded-md px-1 font-sans text-[0.65rem] font-semibold",
                isActive(item.href) ? "text-accent" : "text-muted",
              )}
            >
              <Icon size={20} />
              <span>{item.label}</span>
            </Link>
          );
        })}
      </nav>
    </div>
  );
}
