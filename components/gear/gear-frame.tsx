"use client";

import { useEffect, useMemo, useState, useTransition } from "react";
import { DndContext, MouseSensor, TouchSensor, KeyboardSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, useSortable, arrayMove, sortableKeyboardCoordinates, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Input, Select, Button, PageTabs, Card } from "@/components/ui";
import { PageHeader, PageBody } from "@/components/page-header";
import { Sheet } from "@/components/sheet";
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
  reorderLoadouts,
  deleteLoadout,
  setDefaultLoadout,
  setLoadoutMember,
  setAllLoadoutMembers,
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
  const [more, setMore] = useState(false);
  const [manager, setManager] = useState(false);
  const [reorder, setReorder] = useState(false);
  const [phoneOptions, setPhoneOptions] = useState(false);
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

  const setAllMembers = (present: boolean) => {
    if (!isLoadout) return;
    const lid = selected as number;
    setMember((m) => {
      const next: Record<number, "base" | "worn"> = present ? { ...(m[lid] ?? {}) } : {};
      if (present) for (const g of groups) for (const r of g.rows) {
        if (r.status === "current" && next[r.id] === undefined)
          next[r.id] = r.defaultWeightClass === "worn" ? "worn" : "base";
      }
      return { ...m, [lid]: next };
    });
    start(async () => await setAllLoadoutMembers(lid, present));
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

  const options = <>
    <label className="flex items-center gap-2 text-sm text-muted">Show <Select aria-label="Show gear" value={status} onChange={(e) => setStatus(e.target.value as StatusFilter)}><option value="current">Current</option><option value="retired">Retired</option><option value="all">All</option></Select></label>
    {isLoadout && <><Button variant="ghost" disabled={pending} onClick={() => setAllMembers(true)}>Select all</Button><Button variant="ghost" disabled={pending} onClick={() => setAllMembers(false)}>Deselect all</Button></>}
    <Button variant="ghost" onClick={() => { setPhoneOptions(false); setManager(true); }}>Manage loadouts</Button>
  </>;
  return (
    <>
      <PageHeader title="Gear" actions={<Button variant="header" onClick={() => setDialog({ editing: null })}>Add gear</Button>}>
        <PageTabs tone="header" label="Gear sections" tabs={[{ value: "inventory", label: "Inventory" }, { value: "wishlist", label: "Wishlist", count: wishlist.length || undefined }]} value={tab} onChange={setTab} />
      </PageHeader>
      <PageBody>
      {tab === "inventory" ? <div className="grid min-w-0 gap-5 desk:grid-cols-[minmax(0,1fr)_340px]">
        <aside className="min-w-0 desk:order-2"><BaseBar baseG={agg.baseG} byCategory={agg.byCategory} loadoutControl={<select aria-label="Loadout" value={selected} onChange={(e) => setSelected(e.target.value === "all" ? "all" : Number(e.target.value))} className="min-h-11 max-w-full border-0 bg-transparent font-bold text-ink"><option value="all">All gear</option>{loadouts.map((l) => <option key={l.id} value={l.id}>{l.name}</option>)}</select>} /></aside>
        <div className="min-w-0 space-y-3">
          <div className="flex items-center gap-2"><Input type="search" aria-label="Search gear" placeholder="Search gear" value={q} onChange={(e) => setQ(e.target.value)} /><Button variant="outline" aria-expanded={more || phoneOptions} onClick={() => { if (window.matchMedia("(min-width: 640px)").matches) setMore(!more); else setPhoneOptions(true); }}>More</Button>{reorder && <Button variant="outline" className="sm:hidden" onClick={() => setReorder(false)}>Done</Button>}</div>
          {more && <Card className="hidden flex-wrap items-center gap-2 p-3 sm:flex">{options}</Card>}
          {pending && <span role="status" className="text-xs text-muted">Saving…</span>}
          <GearList groups={shownGroups} onEdit={(r) => setDialog({ editing: r })} onDelete={remove} onMove={(itemId, toCategoryId, orderedIds) => start(async () => { await moveGear(itemId, toCategoryId, orderedIds); })} loadoutMode={isLoadout} memberMap={memberMap} onToggleMember={toggleMember} reorder={reorder} />
        </div>
      </div> : <WishlistFrame wishlist={wishlist} groups={groups} loadouts={loadouts} membership={member} overrides={overrides} pending={pending} onEdit={(r) => setDialog({ editing: r })} onDelete={remove} onSetReplaces={(id, ids) => start(async () => await setWishlistReplacements(id, ids))} onSetWishlistLoadout={setWishInLoadout} />}
      <Sheet open={phoneOptions} onClose={() => setPhoneOptions(false)} title="Gear options"><div className="mt-4 flex flex-col items-start gap-2 [&>button]:justify-start [&>button]:px-0">{options}<Button variant="ghost" onClick={() => { setReorder(true); setPhoneOptions(false); }}>Reorder gear</Button></div></Sheet>
      <LoadoutBar open={manager} onClose={() => setManager(false)} loadouts={loadouts} onCreate={(name) => start(async () => setSelected(await createLoadout(name)))} onRename={(id, name) => start(async () => await renameLoadout(id, name))} onDelete={(id) => start(async () => await deleteLoadout(id))} onSetDefault={(id) => start(async () => await setDefaultLoadout(id))} onReorder={(ids) => start(async () => await reorderLoadouts(ids))} pending={pending} />
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
      </PageBody>
    </>
  );
}

function LoadoutBar({
  loadouts, open, onClose, onCreate, onRename, onDelete, onSetDefault, onReorder, pending,
}: {
  loadouts: LoadoutLite[];
  open: boolean;
  onClose: () => void;
  onCreate: (name: string) => void;
  onRename: (id: number, name: string) => void;
  onDelete: (id: number) => void;
  onSetDefault: (id: number) => void;
  onReorder: (ids: number[]) => void;
  pending: boolean;
}) {
  const [ordered, setOrdered] = useState(loadouts);
  const loadoutSignature = JSON.stringify(loadouts);
  useEffect(() => setOrdered(loadouts), [loadoutSignature]);
  const sensors = useSensors(
    useSensor(MouseSensor, { activationConstraint: { distance: 5 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 180, tolerance: 8 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const move = (from: number, to: number) => {
    if (pending || from < 0 || from === to || to < 0 || to >= ordered.length) return;
    const next = arrayMove(ordered, from, to);
    setOrdered(next);
    onReorder(next.map((l) => l.id));
  };
  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    move(ordered.findIndex((l) => l.id === active.id), ordered.findIndex((l) => l.id === over.id));
  };
  return (
    <Sheet open={open} onClose={onClose} title="Manage loadouts">
            <p className="mb-3 text-sm text-muted">Drag a handle to reorder, or use the move buttons.</p>
            <div className="min-h-0 overflow-y-auto">
              <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
                <SortableContext items={ordered.map((l) => l.id)} strategy={verticalListSortingStrategy}>
                  <div className="space-y-2">
                    {ordered.map((l, index) => (
                      <SortableLoadout key={l.id} loadout={l} index={index} count={ordered.length} pending={pending}
                        onMove={move} onRename={onRename} onDelete={onDelete} onSetDefault={onSetDefault} />
                    ))}
                  </div>
                </SortableContext>
              </DndContext>
              {ordered.length === 0 && <p className="py-4 text-sm text-muted">No loadouts yet.</p>}
            </div>
            <div className="mt-4 flex justify-between gap-2 border-t pt-4">
              <Button disabled={pending} onClick={() => { const name = prompt("New loadout name:"); if (name?.trim()) onCreate(name); }}>+ Add loadout</Button>
              <Button variant="outline" onClick={onClose}>Done</Button>
            </div>
    </Sheet>
  );
}

function SortableLoadout({ loadout, index, count, pending, onMove, onRename, onDelete, onSetDefault }: {
  loadout: LoadoutLite; index: number; count: number; pending: boolean;
  onMove: (from: number, to: number) => void;
  onRename: (id: number, name: string) => void;
  onDelete: (id: number) => void;
  onSetDefault: (id: number) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: loadout.id, disabled: pending });
  return (
    <div ref={setNodeRef} style={{ transform: CSS.Transform.toString(transform), transition }}
      className={`rounded-lg border bg-panel2 p-2 ${isDragging ? "relative z-10 shadow-lg" : ""}`}>
      <div className="flex items-center gap-2">
        <button type="button" {...attributes} {...listeners} disabled={pending}
          aria-label={`Drag to reorder ${loadout.name}`} className="min-h-11 min-w-11 touch-none rounded border text-muted">☰</button>
        <span className="min-w-0 flex-1 truncate text-sm font-medium">{loadout.isDefault && "★ "}{loadout.name}</span>
        <button disabled={pending || index === 0} onClick={() => onMove(index, index - 1)} aria-label={`Move ${loadout.name} up`} className="min-h-11 min-w-9 disabled:opacity-30">↑</button>
        <button disabled={pending || index === count - 1} onClick={() => onMove(index, index + 1)} aria-label={`Move ${loadout.name} down`} className="min-h-11 min-w-9 disabled:opacity-30">↓</button>
      </div>
      <div className="flex flex-wrap items-center gap-3 pl-12 text-xs">
        {!loadout.isDefault && <button disabled={pending} onClick={() => onSetDefault(loadout.id)} className="py-2 text-accent">Set default</button>}
        <button disabled={pending} onClick={() => { const name = prompt("Rename loadout:", loadout.name); if (name?.trim()) onRename(loadout.id, name); }} className="py-2 text-accent">Rename</button>
        <button disabled={pending} onClick={() => { if (confirm(`Delete loadout “${loadout.name}”?`)) onDelete(loadout.id); }} className="py-2 text-accent">Delete</button>
      </div>
    </div>
  );
}
