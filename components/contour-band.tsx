import { cn } from "@/lib/util";

export function ContourBand({ className }: { className?: string }) {
  return (
    <svg aria-hidden="true" className={cn("pointer-events-none absolute inset-0 h-full w-full", className)} viewBox="0 0 1280 200" preserveAspectRatio="xMidYMid slice">
      <path d="M-10 40C200 8 400 76 640 36S1040 6 1290 36M-10 90C210 58 410 126 650 86S1050 56 1290 86M-10 140C220 108 420 176 660 136S1060 106 1290 136M-10 190C230 158 430 226 670 186S1070 156 1290 186" fill="none" stroke="var(--header-line)" strokeWidth={1.2} />
    </svg>
  );
}
