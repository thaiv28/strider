"use client";

import { useMemo, useState } from "react";
import { Input, Select } from "@/components/ui";
import { fmtWeight, fmtUsd, gToOz } from "@/lib/util";
import { categoryColor, OTHER_CATEGORY_NAME } from "@/lib/categories";
import type { GearRow, Group, LoadoutLite, Membership, WishlistOverrides } from "@/lib/gear";

type Target = { id: number; name: string; totalG: number; isBase: boolean; category: string };
type Sel = number | "all";

export function WishlistFrame({
  wishlist,
  groups,
  loadouts,
  membership,
  overrides,
  pending,
  onEdit,
  onDelete,
  onSetReplaces,
  onSetWishlistLoadout,
}: {
  wishlist: GearRow[];
  groups: Group[];
  loadouts: LoadoutLite[];
  membership: Membership;
  overrides: WishlistOverrides;
  pending: boolean;
  onEdit: (r: GearRow) => void;
  onDelete: (r: GearRow) => void;
  onSetReplaces: (wishlistItemId: number, replacesIds: number[]) => void;
  onSetWishlistLoadout: (wishlistItemId: number, loadoutId: number, included: boolean | null) => void;
}) {
  const [q, setQ] = useState("");
  const [sel, setSel] = useState<Sel>(loadouts.find((l) => l.isDefault)?.id ?? loadouts[0]?.id ?? "all");
  const loadoutId = sel === "all" ? null : sel;

  // Current gear is what a wishlist item can replace, and the projection baseline.
  const { targets, targetById } = useMemo(() => {
    const targets: Target[] = [];
    for (const g of groups)
      for (const r of g.rows)
        if (r.status === "current") {
          targets.push({
            id: r.id,
            name: r.name,
            totalG: (r.weightG ?? 0) * r.quantity,
            isBase: r.defaultWeightClass === "base",
            category: r.slotName ?? OTHER_CATEGORY_NAME,
          });
        }
    targets.sort((a, b) => a.category.localeCompare(b.category) || a.name.localeCompare(b.name));
    return { targets, targetById: new Map(targets.map((t) => [t.id, t])) };
  }, [groups]);

  // Assumed member iff any replaced item sits in the loadout; an explicit
  // override wins. "All gear" has no membership — every item counts.
  const effIn = (r: GearRow, lid: number | null): boolean => {
    if (lid === null) return true;
    const ov = overrides[r.id]?.[lid];
    if (ov !== undefined) return ov;
    const lm = membership[lid] ?? {};
    return r.replaces.some((id) => lm[id] !== undefined);
  };

  const addG = (r: GearRow) => (r.defaultWeightClass === "base" ? (r.weightG ?? 0) * r.quantity : 0);
  const removeG = (r: GearRow, lid: number | null) => {
    const lm = lid === null ? null : membership[lid] ?? {};
    return r.replaces.reduce((s, id) => {
      const t = targetById.get(id);
      if (!t) return s;
      if (lm === null) return s + (t.isBase ? t.totalG : 0); // library view: retire base items
      return s + (lm[id] === "base" ? t.totalG : 0); // loadout view: retire base members only
    }, 0);
  };

  // Projection over the items active in the current selection.
  const proj = useMemo(() => {
    let baseline = 0;
    if (loadoutId === null) {
      for (const t of targets) if (t.isBase) baseline += t.totalG;
    } else {
      const lm = membership[loadoutId] ?? {};
      for (const t of targets) if (lm[t.id] === "base") baseline += t.totalG;
    }
    let add = 0;
    let totalCents = 0;
    let count = 0;
    const retired = new Set<number>();
    for (const r of wishlist) {
      if (!effIn(r, loadoutId)) continue;
      count++;
      add += addG(r);
      if (r.priceCents != null) totalCents += r.priceCents * r.quantity;
      for (const id of r.replaces) {
        const inLoadout = loadoutId === null || (membership[loadoutId] ?? {})[id] !== undefined;
        if (inLoadout) retired.add(id);
      }
    }
    let remove = 0;
    for (const id of retired) {
      const t = targetById.get(id);
      if (!t) continue;
      if (loadoutId === null) remove += t.isBase ? t.totalG : 0;
      else remove += (membership[loadoutId] ?? {})[id] === "base" ? t.totalG : 0;
    }
    return { projectedBaseG: Math.max(0, baseline + add - remove), baselineG: baseline, totalCents, count };
  }, [wishlist, targets, targetById, membership, overrides, loadoutId]);

  const shown = useMemo(
    () => wishlist.filter((r) => q === "" || r.name.toLowerCase().includes(q.toLowerCase())),
    [wishlist, q],
  );

  const byCat = useMemo(() => {
    const m = new Map<string, GearRow[]>();
    for (const r of shown) {
      const cat = r.slotName ?? OTHER_CATEGORY_NAME;
      (m.get(cat) ?? m.set(cat, []).get(cat)!).push(r);
    }
    return [...m.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  }, [shown]);

  const selName = loadoutId === null ? "all gear" : loadouts.find((l) => l.id === loadoutId)?.name ?? "";
  const chip = (active: boolean) =>
    `rounded-full border px-3 py-1 text-sm transition ${
      active ? "border-accent bg-accent text-accentink" : "text-muted hover:text-ink"
    }`;

  return (
    <div className="mt-6">
      <div className="flex flex-wrap items-center gap-2">
        <span className="eyebrow mr-1">Project onto</span>
        {loadouts.map((l) => (
          <button key={l.id} onClick={() => setSel(l.id)} className={chip(sel === l.id)}>
            {l.isDefault && "★ "}
            {l.name}
          </button>
        ))}
        <button onClick={() => setSel("all")} className={chip(sel === "all")}>
          All gear
        </button>
      </div>

      <div className="mt-4">
        <ProjectionBar
          scope={selName}
          baselineG={proj.baselineG}
          projectedBaseG={proj.projectedBaseG}
          totalCents={proj.totalCents}
          count={proj.count}
        />
      </div>

      <div className="mt-6 flex flex-wrap items-center gap-2">
        <Input placeholder="Search wishlist…" value={q} onChange={(e) => setQ(e.target.value)} className="max-w-xs" />
        {pending && <span className="eyebrow">saving…</span>}
      </div>

      {shown.length === 0 ? (
        <div className="card mt-4 px-4 py-12 text-center text-sm text-muted">
          Nothing on the wishlist yet — add an item and set its status to “wishlist”.
        </div>
      ) : (
        <div className="mt-4 space-y-3">
          {byCat.map(([cat, rows]) => (
            <div key={cat} className="card overflow-hidden">
              <div className="flex items-center gap-2 border-b bg-panel2/60 px-4 py-2.5">
                <span className="h-2.5 w-2.5 rounded-sm" style={{ background: categoryColor(cat) }} />
                <span className="font-display font-semibold">{cat}</span>
              </div>
              {rows.map((r) => (
                <WishRow
                  key={r.id}
                  r={r}
                  targets={targets}
                  targetById={targetById}
                  loadoutId={loadoutId}
                  active={effIn(r, loadoutId)}
                  overridden={loadoutId !== null && overrides[r.id]?.[loadoutId] !== undefined}
                  swingG={effIn(r, loadoutId) ? addG(r) - removeG(r, loadoutId) : 0}
                  onEdit={onEdit}
                  onDelete={onDelete}
                  onSetReplaces={onSetReplaces}
                  onSetWishlistLoadout={onSetWishlistLoadout}
                />
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ProjectionBar({
  scope,
  baselineG,
  projectedBaseG,
  totalCents,
  count,
}: {
  scope: string;
  baselineG: number;
  projectedBaseG: number;
  totalCents: number;
  count: number;
}) {
  const delta = projectedBaseG - baselineG;
  const saves = delta < 0;
  const deltaColor = saves ? "text-cbase" : delta > 0 ? "text-ccons" : "text-muted";
  return (
    <div className="card flex flex-wrap items-center justify-between gap-4 p-5">
      <div>
        <span className="eyebrow">◇ Base weight change — {scope}</span>
        <div className="mt-1 flex items-baseline gap-2">
          <span className={`readout text-4xl font-bold leading-none ${deltaColor}`}>
            {delta === 0 ? "±0" : `${saves ? "−" : "+"}${gToOz(Math.abs(delta)).toFixed(1)}`}
          </span>
          <span className="text-lg text-muted">oz</span>
        </div>
        <div className="readout mt-1 text-sm text-muted">
          {fmtWeight(baselineG)} <span className="mx-0.5">→</span> {fmtWeight(projectedBaseG)}
        </div>
      </div>
      <div className="readout text-right text-sm text-muted">
        {count} item{count === 1 ? "" : "s"}
        {totalCents > 0 && (
          <div className="mt-0.5 font-semibold text-ink">{fmtUsd(totalCents)}</div>
        )}
      </div>
    </div>
  );
}

function WishRow({
  r,
  targets,
  targetById,
  loadoutId,
  active,
  overridden,
  swingG,
  onEdit,
  onDelete,
  onSetReplaces,
  onSetWishlistLoadout,
}: {
  r: GearRow;
  targets: Target[];
  targetById: Map<number, Target>;
  loadoutId: number | null;
  active: boolean;
  overridden: boolean;
  swingG: number;
  onEdit: (r: GearRow) => void;
  onDelete: (r: GearRow) => void;
  onSetReplaces: (wishlistItemId: number, replacesIds: number[]) => void;
  onSetWishlistLoadout: (wishlistItemId: number, loadoutId: number, included: boolean | null) => void;
}) {
  const totalG = r.weightG == null ? null : r.weightG * r.quantity;
  const addTarget = (id: number) => onSetReplaces(r.id, [...r.replaces, id]);
  const removeTarget = (id: number) => onSetReplaces(r.id, r.replaces.filter((x) => x !== id));
  const available = targets.filter((t) => t.id !== r.id && !r.replaces.includes(t.id));

  return (
    <div className={`border-b px-4 py-2.5 last:border-0 hover:bg-panel2/40 ${loadoutId !== null && !active ? "opacity-55" : ""}`}>
      <div className="flex items-baseline gap-2">
        {loadoutId !== null && (
          <input
            type="checkbox"
            checked={active}
            onChange={(e) => onSetWishlistLoadout(r.id, loadoutId, e.target.checked)}
            aria-label={`Include ${r.name} in this loadout`}
            className="h-4 w-4 shrink-0 translate-y-0.5 accent-[var(--accent)]"
          />
        )}
        <button onClick={() => onEdit(r)} className="flex min-w-0 items-baseline gap-1.5 text-left transition hover:text-accent">
          <span className="truncate font-medium">{r.name}</span>
          {r.quantity > 1 && <span className="shrink-0 text-sm text-muted">×{r.quantity}</span>}
          {r.defaultWeightClass === "worn" && <span className="shrink-0 text-xs text-muted">· worn</span>}
        </button>
        <span className="readout ml-auto shrink-0 text-sm text-muted">{fmtWeight(totalG)}</span>
        <span className="readout w-16 shrink-0 text-right text-sm text-muted">
          {fmtUsd(r.priceCents == null ? null : r.priceCents * r.quantity)}
        </span>
        <button
          onClick={() => onDelete(r)}
          aria-label={`Delete ${r.name}`}
          className="shrink-0 rounded p-1 text-muted transition hover:bg-accent/10 hover:text-accent"
        >
          ✕
        </button>
      </div>

      <div className="mt-1.5 flex flex-wrap items-center gap-1.5 text-xs">
        <span className="text-muted">replaces</span>
        {r.replaces.map((id) => {
          const t = targetById.get(id);
          return (
            <span key={id} className="inline-flex items-center gap-1 rounded-full border bg-panel2/50 px-2 py-0.5">
              {t?.name ?? "unknown"}
              {t && <span className="text-muted">{fmtWeight(t.totalG)}</span>}
              <button onClick={() => removeTarget(id)} aria-label="Remove" className="text-muted hover:text-accent">
                ✕
              </button>
            </span>
          );
        })}
        {available.length > 0 && (
          <Select
            value=""
            onChange={(e) => e.target.value && addTarget(Number(e.target.value))}
            className="rounded-full py-0.5 text-xs leading-none"
          >
            <option value="">+ add…</option>
            {available.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name} ({fmtWeight(t.totalG)})
              </option>
            ))}
          </Select>
        )}
        {loadoutId !== null && overridden && (
          <button
            onClick={() => onSetWishlistLoadout(r.id, loadoutId, null)}
            className="text-muted hover:text-accent"
            title="Revert to the assumed membership"
          >
            (auto)
          </button>
        )}
        {active && swingG !== 0 && (
          <span className={`readout ml-1 font-medium ${swingG < 0 ? "text-cbase" : "text-ccons"}`}>
            {swingG < 0 ? "↓" : "↑"} {gToOz(Math.abs(swingG)).toFixed(1)} oz base
          </span>
        )}
      </div>
    </div>
  );
}
