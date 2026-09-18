import * as React from "react";
import { cn } from "@/lib/util";

export function Card({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("card", className)} {...props} />;
}

const badgeTones: Record<string, string> = {
  base: "border-cbase/30 bg-cbase/10 text-cbase",
  worn: "border-cworn/40 bg-cworn/15 text-cworn",
  consumable: "border-ccons/30 bg-ccons/10 text-ccons",
  current: "border-cbase/30 bg-cbase/10 text-cbase",
  retired: "border-line bg-panel2 text-muted",
  wishlist: "border-accent/30 bg-accent/10 text-accent",
  neutral: "border-line bg-panel2 text-muted",
};

// A circled "i" that reveals a tooltip on hover/focus — for tucking away
// explanatory detail so the surrounding UI stays uncluttered.
export function InfoBadge({
  children,
  side = "bottom",
  className,
}: {
  children: React.ReactNode;
  side?: "top" | "bottom";
  className?: string;
}) {
  return (
    <span className={cn("group relative inline-flex align-middle", className)}>
      <button
        type="button"
        aria-label="More info"
        className="flex h-4 w-4 items-center justify-center rounded-full border border-muted/50 text-[0.6rem] font-bold leading-none text-muted transition hover:border-accent hover:text-accent focus:outline-none focus-visible:border-accent focus-visible:text-accent"
      >
        i
      </button>
      {/* Outer wrapper carries a transparent padding "bridge" so moving the
          cursor from the icon onto the tooltip never crosses a dead gap that
          would drop the hover. Interactive only while shown, so the text is
          selectable/copyable. */}
      <span
        role="tooltip"
        className={cn(
          "pointer-events-none fixed inset-x-4 bottom-[calc(5rem+env(safe-area-inset-bottom))] z-[1200] w-auto translate-x-0 opacity-0 transition-opacity duration-100 group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100 sm:absolute sm:inset-x-auto sm:bottom-auto sm:left-1/2 sm:w-64 sm:-translate-x-1/2",
          side === "bottom" ? "sm:top-full sm:pt-1.5" : "sm:bottom-full sm:pb-1.5",
        )}
      >
        <span className="block cursor-text select-text rounded-lg border bg-panel px-3 py-2 text-xs font-normal leading-relaxed normal-case tracking-normal text-muted shadow-lg">
          {children}
        </span>
      </span>
    </span>
  );
}

export function Badge({ tone = "neutral", children }: { tone?: string; children: React.ReactNode }) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full border px-2 py-0.5 text-[0.6875rem] font-medium tracking-wide uppercase",
        badgeTones[tone] ?? badgeTones.neutral,
      )}
    >
      {children}
    </span>
  );
}

export function Button({
  variant = "primary",
  className,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "ghost" | "outline" | "danger";
}) {
  const styles = {
    primary: "bg-accent text-accentink hover:opacity-90",
    outline: "border bg-panel hover:bg-panel2",
    ghost: "text-muted hover:bg-panel2 hover:text-ink",
    danger: "text-accent hover:bg-accent/10",
  }[variant];
  return (
    <button
      className={cn(
        "inline-flex min-h-11 items-center justify-center gap-1.5 rounded-[calc(var(--radius)*0.6)] px-3 py-1.5 text-sm font-medium transition disabled:opacity-50 sm:min-h-0",
        styles,
        className,
      )}
      {...props}
    />
  );
}

export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "min-w-0 max-w-full w-full rounded-[calc(var(--radius)*0.6)] border bg-panel2 px-3 py-1.5 text-sm outline-none focus:border-accent/50 focus:ring-2 focus:ring-accent/25",
        className,
      )}
      {...props}
    />
  ),
);
Input.displayName = "Input";

export function WornToggle({
  value,
  onChange,
}: {
  value: "base" | "worn";
  onChange: (v: "base" | "worn") => void;
}) {
  return (
    <span className="inline-flex overflow-hidden rounded-full border text-xs">
      {(["base", "worn"] as const).map((v) => (
        <button
          key={v}
          onClick={() => onChange(v)}
          className={cn(
            "w-11 py-0.5 text-center transition",
            v === value ? "bg-accent font-medium text-accentink" : "text-muted hover:text-ink",
          )}
        >
          {v === "base" ? "pack" : "worn"}
        </button>
      ))}
    </span>
  );
}

export const Select = React.forwardRef<
  HTMLSelectElement,
  React.SelectHTMLAttributes<HTMLSelectElement>
>(({ className, ...props }, ref) => (
  <select
    ref={ref}
    className={cn(
      "rounded-[calc(var(--radius)*0.6)] border bg-panel2 px-3 py-1.5 text-sm outline-none focus:border-accent/50 focus:ring-2 focus:ring-accent/25",
      className,
    )}
    {...props}
  />
));
Select.displayName = "Select";
