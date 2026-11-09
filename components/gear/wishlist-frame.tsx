"use client";

import { useMemo, useState } from "react";
import { Input, Select, Button, IconButton } from "@/components/ui";
import { Sheet } from "@/components/sheet";
import { MoreHorizontal } from "lucide-react";
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
    let priceKnown = true;
    const retired = new Set<number>();
    for (const r of wishlist) {
      if (!effIn(r, loadoutId)) continue;
      count++;
      add += addG(r);
      if (r.priceCents != null) totalCents += r.priceCents * r.quantity;
      else priceKnown = false;
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
    return { projectedBaseG: Math.max(0, baseline + add - remove), baselineG: baseline, totalCents, count, priceKnown };
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

  const savingOz = gToOz(proj.baselineG - proj.projectedBaseG);
  return (
    <div className="grid min-w-0 gap-5 desk:grid-cols-[minmax(0,1fr)_340px]">
      <aside className="min-w-0 desk:order-2">
        <div className="card p-4">
          <label className="flex flex-wrap items-center gap-2 text-sm text-muted">If added to <select aria-label="Wishlist loadout" value={sel} onChange={(e) => setSel(e.target.value === "all" ? "all" : Number(e.target.value))} className="min-h-11 min-w-0 max-w-full border-0 bg-transparent font-bold text-ink"><option value="all">All gear</option>{loadouts.map((l) => <option key={l.id} value={l.id}>{l.name}</option>)}</select></label>
          <div className="font-display text-[32px] font-bold leading-tight tabular-nums">{(gToOz(proj.baselineG) / 16).toFixed(1)} → {(gToOz(proj.projectedBaseG) / 16).toFixed(1)} lb</div>
          {savingOz > 0 && <><div className="mt-2 font-bold text-signal">Saves {savingOz.toFixed(1)} oz{proj.priceKnown && ` for ${fmtUsd(proj.totalCents)}`}</div>{proj.priceKnown && <div className="mt-1 text-sm text-muted">About {fmtUsd(Math.round(proj.totalCents / savingOz))} per ounce saved</div>}</>}
          <div className="mt-2 text-xs text-muted">{proj.count} item{proj.count === 1 ? "" : "s"}</div>
        </div>
      </aside>
      <div className="min-w-0">
        <Input type="search" aria-label="Search wishlist" placeholder="Search wishlist" value={q} onChange={(e) => setQ(e.target.value)} />
        {pending && <span role="status" className="text-xs text-muted">Saving…</span>}
      {shown.length === 0 ? (
        <div className="card mt-4 px-4 py-12 text-center text-sm text-muted">
          Nothing on the wishlist yet — add an item and set its status to “wishlist”.
        </div>
      ) : (
        <div className="mt-4 space-y-3">
          {byCat.map(([cat, rows]) => (
            <div key={cat} className="card">
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
                  swingG={(r.weightG ?? 0) * r.quantity - r.replaces.reduce((sum, id) => sum + (targetById.get(id)?.totalG ?? 0), 0)}
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
  const [menu, setMenu] = useState(false);
  const [sheet, setSheet] = useState(false);
  const [replacements, setReplacements] = useState(false);
  const close = () => { setMenu(false); setSheet(false); };
  const available = targets.filter((t) => t.id !== r.id && !r.replaces.includes(t.id));
  const actions = <>
    <Button variant="ghost" className="w-full justify-start" onClick={() => { close(); onEdit(r); }}>Edit</Button>
    <Button variant="ghost" className="w-full justify-start" onClick={() => { close(); setReplacements(true); }}>Edit replacements</Button>
    {loadoutId !== null && overridden && <Button variant="ghost" className="w-full justify-start" onClick={() => { close(); onSetWishlistLoadout(r.id, loadoutId, null); }}>Use automatic selection</Button>}
    <Button variant="danger" className="w-full justify-start" onClick={() => { close(); onDelete(r); }}>Delete</Button>
  </>;
  return (
    <div className={`relative flex items-center gap-2 border-b px-3 py-2 last:border-0 sm:px-4 ${loadoutId !== null && !active ? "opacity-55" : ""}`}>
      {loadoutId !== null && <label className="grid min-h-11 w-7 shrink-0 place-items-center"><input type="checkbox" checked={active} onChange={(e) => onSetWishlistLoadout(r.id, loadoutId, e.target.checked)} aria-label={`Try ${r.name} in this loadout`} className="h-5 w-5 accent-[var(--accent)]" /></label>}
      <button className="min-h-11 min-w-0 flex-1 text-left" onClick={() => { if (window.matchMedia("(min-width: 640px)").matches) onEdit(r); else setSheet(true); }}>
        <span className="block truncate font-semibold">{r.name}{r.quantity > 1 && ` ×${r.quantity}`}</span>
        <span className="block truncate text-xs text-muted">{r.replaces.length ? `Replaces ${r.replaces.map((id) => targetById.get(id)?.name ?? "unknown").join(", ")}` : "New item"}</span>
      </button>
      <button tabIndex={-1} onClick={() => setSheet(true)} className="shrink-0 text-right text-sm tabular-nums sm:pointer-events-none"><span className={`block font-semibold ${swingG < 0 ? "text-signal" : "text-ink"}`}>{r.weightG === null ? "—" : `${swingG < 0 ? "−" : "+"}${gToOz(Math.abs(swingG)).toFixed(1)} oz`}</span><span className="block text-xs text-muted">{fmtUsd(r.priceCents == null ? null : r.priceCents * r.quantity)}</span></button>
      <div className="relative hidden sm:block"><IconButton aria-label={`More options for ${r.name}`} aria-expanded={menu} onClick={() => setMenu(!menu)}><MoreHorizontal size={18} /></IconButton>{menu && <><button className="fixed inset-0 z-20 cursor-default" aria-label="Close options" onClick={close} /><div role="group" aria-label={`Options for ${r.name}`} className="absolute right-0 top-full z-30 w-52 rounded-xl border bg-panel p-1 shadow-lg" onKeyDown={(e) => { if (e.key === "Escape") close(); }}>{actions}</div></>}</div>
      <Sheet open={sheet} onClose={close} title={r.name}><div className="mt-3">{actions}</div></Sheet>
      <Sheet open={replacements} onClose={() => setReplacements(false)} title={`Replacements for ${r.name}`}>
        <div className="mt-4 space-y-2">
          {r.replaces.map((id) => <div key={id} className="flex items-center justify-between gap-2 text-sm"><span>{targetById.get(id)?.name ?? "unknown"}</span><Button variant="ghost" aria-label={`Remove ${targetById.get(id)?.name ?? "replacement"}`} onClick={() => onSetReplaces(r.id, r.replaces.filter((x) => x !== id))}>Remove</Button></div>)}
          <Select aria-label="Add replacement" className="w-full" value="" onChange={(e) => { if (e.target.value) onSetReplaces(r.id, [...r.replaces, Number(e.target.value)]); }}><option value="">Add replacement…</option>{available.map((t) => <option key={t.id} value={t.id}>{t.name} ({fmtWeight(t.totalG)})</option>)}</Select>
        </div>
      </Sheet>
    </div>
  );
}
