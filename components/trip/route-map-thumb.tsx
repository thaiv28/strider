"use client";

import dynamic from "next/dynamic";

const RouteMiniMap = dynamic(() => import("./route-minimap"), { ssr: false });

export function RouteMapThumb({ track, className }: { track: [number, number][]; className?: string }) {
  if (track.length < 2) return null;
  return (
    <div className={`pointer-events-none relative isolate z-0 overflow-hidden rounded-md border ${className ?? ""}`}>
      <RouteMiniMap track={track} className="h-full w-full" />
    </div>
  );
}
