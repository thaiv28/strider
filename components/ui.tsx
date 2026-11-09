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
        "inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-semibold",
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
  variant?: "primary" | "ghost" | "outline" | "danger" | "header";
}) {
  const styles = {
    primary: "bg-accent text-accentink hover:opacity-90",
    outline: "border border-fieldborder bg-panel text-ink hover:bg-panel2",
    ghost: "text-muted hover:bg-panel2 hover:text-ink",
    danger: "text-signal hover:bg-signal/10",
    header: "bg-headerink text-headeractionink hover:opacity-90 focus-visible:outline-headerink",
  }[variant];
  return (
    <button
      className={cn(
        "inline-flex min-h-11 items-center justify-center gap-1.5 rounded-xl px-4 py-1.5 text-sm font-semibold transition disabled:opacity-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
        styles,
        className,
      )}
      {...props}
    />
  );
}

export function PageTabs<T extends string>({
  label,
  tabs,
  tone = "page",
  value,
  onChange,
  className,
}: {
  label: string;
  tabs: readonly { value: T; label: string; count?: number }[];
  tone?: "page" | "header";
  value: T;
  onChange: (value: T) => void;
  className?: string;
}) {
  const selectByKey = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    let next: number;
    if (event.key === "ArrowRight") next = (index + 1) % tabs.length;
    else if (event.key === "ArrowLeft") next = (index - 1 + tabs.length) % tabs.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = tabs.length - 1;
    else return;
    event.preventDefault();
    onChange(tabs[next].value);
    event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>("[role=tab]")[next]?.focus();
  };

  return (
    <div role="tablist" aria-label={label} className={cn("flex flex-wrap gap-1.5", className)}>
      {tabs.map((tab, index) => (
        <button
          key={tab.value}
          type="button"
          role="tab"
          aria-selected={value === tab.value}
          tabIndex={value === tab.value ? 0 : -1}
          onClick={() => onChange(tab.value)}
          onKeyDown={(event) => selectByKey(event, index)}
          className={cn(
            "min-h-11 rounded-xl px-4 text-sm font-semibold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2",
            tone === "header" ? "focus-visible:outline-headerink" : "focus-visible:outline-accent",
            tone === "header"
              ? value === tab.value
                ? "border border-headerink bg-headerink text-headeractionink"
                : "border border-headerborder text-headerink hover:bg-headeractive"
              : value === tab.value
                ? "border border-accent bg-accent text-accentink"
                : "border border-fieldborder bg-panel text-ink hover:bg-panel2",
          )}
        >
          {tab.label}
          {tab.count != null && <span className="ml-1.5 text-xs font-normal">({tab.count})</span>}
        </button>
      ))}
    </div>
  );
}

export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "min-w-0 max-w-full w-full min-h-11 rounded-[10px] border border-fieldborder bg-panel px-3 text-sm text-ink outline-none focus:border-accent/60 focus:ring-2 focus:ring-accent/25",
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
      "min-h-11 rounded-[10px] border border-fieldborder bg-panel px-3 text-sm text-ink outline-none focus:border-accent/60 focus:ring-2 focus:ring-accent/25",
      className,
    )}
    {...props}
  />
));
Select.displayName = "Select";

export const Textarea = React.forwardRef<HTMLTextAreaElement, React.TextareaHTMLAttributes<HTMLTextAreaElement>>(
  ({ className, ...props }, ref) => (
    <textarea ref={ref} className={cn("rounded-[10px] border border-fieldborder bg-panel px-3 py-2 text-sm text-ink outline-none focus:border-accent/60 focus:ring-2 focus:ring-accent/25", className)} {...props} />
  ),
);
Textarea.displayName = "Textarea";

export function Field({ label, hint, className, children }: { label: string; hint?: React.ReactNode; className?: string; children: React.ReactNode }) {
  return (
    <label className={cn("block", className)}>
      <span className="mb-1 block text-[13px] font-semibold text-muted">{label}</span>
      {children}
      {hint && <span className="block text-[13px] text-muted">{hint}</span>}
    </label>
  );
}

export function Chip({ selected = false, type = "button", className, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement> & { selected?: boolean }) {
  return <button {...props} type={type} aria-pressed={selected} className={cn(
    "min-h-11 shrink-0 rounded-xl border px-3.5 text-sm font-semibold transition",
    selected ? "border-accent bg-accent text-accentink" : "border-fieldborder bg-panel text-ink hover:bg-panel2", className,
  )} />;
}

export function IconButton({ surface = false, type = "button", className, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement> & { "aria-label": string; surface?: boolean }) {
  return <button type={type} className={cn(
    "grid h-11 w-11 shrink-0 place-items-center rounded-xl transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
    surface ? "bg-panel text-ink hover:bg-panel2" : "text-muted hover:bg-panel2 hover:text-ink", className,
  )} {...props} />;
}

export function SegmentedControl<T extends string>({ label, options, value, onChange, className }: {
  label: string;
  options: readonly { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
  className?: string;
}) {
  const selectByKey = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    let next: number;
    if (event.key === "ArrowRight" || event.key === "ArrowDown") next = (index + 1) % options.length;
    else if (event.key === "ArrowLeft" || event.key === "ArrowUp") next = (index - 1 + options.length) % options.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = options.length - 1;
    else return;
    event.preventDefault();
    onChange(options[next].value);
    event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>("[role=radio]")[next]?.focus();
  };
  return (
    <div role="radiogroup" aria-label={label} className={cn("grid gap-0.5 rounded-xl bg-canvas p-[3px]", className)} style={{ gridTemplateColumns: `repeat(${options.length}, minmax(0, 1fr))` }}>
      {options.map((option, index) => (
        <button key={option.value} type="button" role="radio" aria-checked={value === option.value} tabIndex={value === option.value ? 0 : -1}
          onClick={() => onChange(option.value)} onKeyDown={(event) => selectByKey(event, index)}
          className={cn("min-h-11 rounded-[9px] text-sm font-semibold transition focus-visible:outline-2 focus-visible:outline-accent", value === option.value ? "bg-panel text-ink shadow-sm" : "text-muted hover:text-ink")}>
          {option.label}
        </button>
      ))}
    </div>
  );
}
