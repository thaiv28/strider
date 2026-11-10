import type { ReactNode } from "react";
import { cn } from "@/lib/util";
import { ContourBand } from "@/components/contour-band";

export function PageHeader({ title, actions, children }: { title: string; actions?: ReactNode; children?: ReactNode }) {
  return (
    <header data-noprint className="relative overflow-hidden bg-header text-headerink">
      <ContourBand />
      <div className="relative mx-auto max-w-[1136px] px-4 pb-12 sm:px-6 sm:pb-16">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h1 className="font-display text-[32px] font-bold leading-[1.05] sm:text-[44px]">{title}</h1>
          {actions}
        </div>
        {children && <div className="mt-3">{children}</div>}
      </div>
    </header>
  );
}

export function PageBody({ className, children }: { className?: string; children: ReactNode }) {
  return <div className={cn("relative mx-auto -mt-9 max-w-[1136px] px-4 pb-8 sm:px-6", className)}>{children}</div>;
}
