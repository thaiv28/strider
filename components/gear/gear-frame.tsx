"use client";

import { useEffect, useMemo, useState, useTransition } from "react";
import { Input, Select, Button } from "@/components/ui";
import { BaseBar } from "./base-bar";
import { GearList } from "./gear-list";
import { GearDialog } from "./gear-dialog";
import { WishlistFrame } from "./wishlist-frame";
import {
  createGear,
  updateGear,
  deleteGear,
  moveGear,
  createLoadout,
  renameLoadout,
  deleteLoadout,
  setDefaultLoadout,
  setLoadoutMember,
  setWishlistReplacements,
  setWishlistLoadout,
} from "@/app/gear/actions";
import type { GearRow, Group, CategoryOption, LoadoutLite, Membership, WishlistOverrides } from "@/lib/gear";

type StatusFilter = "all" | "current" | "retired";
type Sel = number | "all";
type Tab = "inventory" | "wishlist";

export function GearFrame({
  groups,
  wishlist,
  categoryOptions,
  loadouts,
  membership,
  wishlistOverrides,
}: {
  groups: Group[];
  wishlist: GearRow[];
  categoryOptions: CategoryOption[];
  loadouts: LoadoutLite[];
  membership: Membership;
  wishlistOverrides: WishlistOverrides;
}) {
  const defaultLoadout = loadouts.find((l) => l.isDefault)?.id ?? loadouts[0]?.id ?? null;
  const [selected, setSelected] = useState<Sel>(defaultLoadout ?? "all");
  const [member, setMember] = useState<Membership>(membership);
  const [overrides, setOverrides] = useState<WishlistOverrides>(wishlistOverrides);
  const [q, setQ] = useState("");
  const [status, setStatus] = useState<StatusFilter>("current");
  const [tab, setTab] = useState<Tab>("inventory");
  const [dialog, setDialog] = useState<{ editing: GearRow | null } | null>(null);
  const [pending, start] = useTransition();

  const memberSig = JSON.stringify(membership);
  useEffect(() => setMember(membership), [memberSig]);
  const overrideSig = JSON.stringify(wishlistOverrides);
  useEffect(() => setOverrides(wishlistOverrides), [overrideSig]);
  // Keep selection valid if a loadout was deleted.
  useEffect(() => {
    if (selected !== "all" && !loadouts.some((l) => l.id === selected))
      setSelected(defaultLoadout ?? "all");
  }, [loadouts, selected, defaultLoadout]);

  const isLoadout = selected !== "all";
  const memberMap = isLoadout ? (member[selected as number] ?? {}) : null;

  const included = (r: GearRow) =>
    r.status === "current" && (memberMap ? memberMap[r.id] !== undefined : true);
  const effClass = (r: GearRow): "base" | "worn" | "consumable" =>
    memberMap ? (memberMap[r.id] ?? "base") : r.defaultWeightClass;

  // Per-category subtotals + base breakdown for the current selection.
  const agg = useMemo(() => {
    const byCat = new Map<number, { name: string; base: number; cons: number; worn: number; items: { name: string; g: number; quantity: number }[] }>();
    for (const g of groups) {
      byCat.set(g.id, { name: g.name, base: 0, cons: 0, worn: 0, items: [] });
      for (const r of g.rows) {
        if (!included(r)) continue;
        const cls = effClass(r);
        const w = (r.weightG ?? 0) * r.quantity;
        byCat.get(g.id)![cls === "consumable" ? "cons" : cls] += w;
        if (cls === "base") byCat.get(g.id)!.items.push({ name: r.name, g: w, quantity: r.quantity });
      }
    }
    const byCategory = [...byCat.values()].map((c) => ({ name: c.name, g: c.base, items: c.items })).filter((c) => c.g > 0);
    const baseG = byCategory.reduce((s, c) => s + c.g, 0);
    return { byCat, byCategory, baseG };
  }, [groups, selected, member]);

  // Wishlist lives on its own tab; the inventory list never shows it.
  const matches = (r: GearRow) =>
    r.status !== "wishlist" &&
    (status === "all" || r.status === status) &&
    (q === "" ||
      r.name.toLowerCase().includes(q.toLowerCase()) ||
      (r.slotName ?? "").toLowerCase().includes(q.toLowerCase()));

  const shownGroups: Group[] = useMemo(
    () =>
      groups
        .map((g) => {
          const a = agg.byCat.get(g.id)!;
          return { ...g, rows: g.rows.filter(matches), baseG: a.base, consumableG: a.cons, wornG: a.worn };
        })
        .filter((g) => g.rows.length > 0),
    [groups, q, status, agg],
  );

  const remove = (r: GearRow) => {
    if (!confirm(`Delete "${r.name}"?`)) return;
    start(async () => {
      await deleteGear(r.id);
    });
  };

  const toggleMember = (itemId: number, present: boolean, cls: "base" | "worn" = "base") => {
    if (!isLoadout) return;
    const lid = selected as number;
    setMember((m) => {
      const next = { ...m, [lid]: { ...(m[lid] ?? {}) } };
      if (present) next[lid][itemId] = cls;
      else delete next[lid][itemId];
      return next;
    });
    start(async () => {
      await setLoadoutMember(lid, itemId, present, cls);
    });
  };

  const setWishInLoadout = (itemId: number, loadoutId: number, included: boolean | null) => {
    setOverrides((o) => {
      const next = { ...o, [itemId]: { ...(o[itemId] ?? {}) } };
      if (included === null) delete next[itemId][loadoutId];
      else next[itemId][loadoutId] = included;
      return next;
    });
    start(async () => {
      await setWishlistLoadout(itemId, loadoutId, included);
    });
  };

  return (
    <div className="mx-auto max-w-5xl px-4 py-6 sm:px-8 sm:py-10">
      <div className="flex flex-wrap items-end justify-between gap-3 sm:gap-4">
        <div>
          <div className="eyebrow">◇ Inventory</div>
          <h1 className="font-display mt-1 text-3xl font-bold tracking-tight">Gear Library</h1>
        </div>
        <Button onClick={() => setDialog({ editing: null })}>
          {tab === "wishlist" ? "+ Add item" : "+ Add gear"}
        </Button>
      </div>

      <div className="mt-6 flex gap-1 border-b">
        {(["inventory", "wishlist"] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`-mb-px border-b-2 px-3 py-1.5 text-sm capitalize transition ${
              tab === t ? "border-accent font-semibold text-ink" : "border-transparent text-muted hover:text-ink"
            }`}
          >
            {t}
            {t === "wishlist" && wishlist.length > 0 && (
              <span className="ml-1.5 text-xs text-muted">{wishlist.length}</span>
            )}
          </button>
        ))}
      </div>

      {tab === "inventory" ? (
        <>
          <LoadoutBar
            loadouts={loadouts}
            selected={selected}
            onSelect={setSelected}
            onCreate={(name) => start(async () => setSelected(await createLoadout(name)))}
            onRename={(id, name) => start(async () => await renameLoadout(id, name))}
            onDelete={(id) => start(async () => await deleteLoadout(id))}
            onSetDefault={(id) => start(async () => await setDefaultLoadout(id))}
          />

          <div className="mt-4">
            <BaseBar baseG={agg.baseG} byCategory={agg.byCategory} />
          </div>

          <div className="mt-8 flex flex-wrap items-center gap-2">
            <Input placeholder="Search gear…" value={q} onChange={(e) => setQ(e.target.value)} className="max-w-xs" />
            <Select value={status} onChange={(e) => setStatus(e.target.value as StatusFilter)}>
              <option value="current">Current</option>
              <option value="retired">Retired</option>
              <option value="all">All</option>
            </Select>
            {pending && <span className="eyebrow">saving…</span>}
          </div>

          <div className="mt-4">
            <GearList
              groups={shownGroups}
              onEdit={(r) => setDialog({ editing: r })}
              onDelete={remove}
              onMove={(itemId, toCategoryId, orderedIds) =>
                start(async () => {
                  await moveGear(itemId, toCategoryId, orderedIds);
                })
              }
              loadoutMode={isLoadout}
              memberMap={memberMap}
              onToggleMember={toggleMember}
            />
          </div>
        </>
      ) : (
        <WishlistFrame
          wishlist={wishlist}
          groups={groups}
          loadouts={loadouts}
          membership={member}
          overrides={overrides}
          pending={pending}
          onEdit={(r) => setDialog({ editing: r })}
          onDelete={remove}
          onSetReplaces={(id, ids) => start(async () => await setWishlistReplacements(id, ids))}
          onSetWishlistLoadout={setWishInLoadout}
        />
      )}

      {dialog && (
        <GearDialog
          editing={dialog.editing}
          categoryOptions={categoryOptions}
          onClose={() => setDialog(null)}
          onSubmit={(fd) =>
            start(async () => {
              if (dialog.editing) await updateGear(dialog.editing.id, fd);
              else await createGear(fd);
              setDialog(null);
            })
          }
        />
      )}
    </div>
  );
}

function LoadoutBar({
  loadouts,
  selected,
  onSelect,
  onCreate,
  onRename,
  onDelete,
  onSetDefault,
}: {
  loadouts: LoadoutLite[];
  selected: Sel;
  onSelect: (s: Sel) => void;
  onCreate: (name: string) => void;
  onRename: (id: number, name: string) => void;
  onDelete: (id: number) => void;
  onSetDefault: (id: number) => void;
}) {
  const chip = (active: boolean) =>
    `rounded-full border px-3 py-1 text-sm transition ${
      active ? "border-accent bg-accent text-accentink" : "text-muted hover:text-ink"
    }`;
  const cur = selected !== "all" ? loadouts.find((l) => l.id === selected) : null;
  return (
    <div className="mt-6 flex flex-wrap items-center gap-2">
      <span className="eyebrow mr-1">Loadout</span>
      {loadouts.map((l) => (
        <button key={l.id} onClick={() => onSelect(l.id)} className={chip(selected === l.id)}>
          {l.isDefault && "★ "}
          {l.name}
        </button>
      ))}
      <button onClick={() => onSelect("all")} className={chip(selected === "all")}>
        All gear
      </button>
      <button
        onClick={() => {
          const n = prompt("New loadout name:");
          if (n) onCreate(n);
        }}
        className="rounded-full px-2 py-1 text-sm text-muted hover:text-ink"
      >
        + New
      </button>
      {cur && (
        <span className="ml-2 flex items-center gap-2 text-xs text-muted">
          {!cur.isDefault && (
            <button onClick={() => onSetDefault(cur.id)} className="hover:text-accent">
              set default
            </button>
          )}
          <button
            onClick={() => {
              const n = prompt("Rename loadout:", cur.name);
              if (n) onRename(cur.id, n);
            }}
            className="hover:text-accent"
          >
            rename
          </button>
          <button
            onClick={() => confirm(`Delete loadout “${cur.name}”?`) && onDelete(cur.id)}
            className="hover:text-accent"
          >
            delete
          </button>
        </span>
      )}
    </div>
  );
}
