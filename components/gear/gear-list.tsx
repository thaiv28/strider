"use client";

import { useEffect, useState } from "react";
import {
  DndContext,
  PointerSensor,
  KeyboardSensor,
  useSensor,
  useSensors,
  closestCenter,
  useDroppable,
  type DragEndEvent,
  type DragOverEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  arrayMove,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Button, IconButton } from "@/components/ui";
import { Sheet } from "@/components/sheet";
import { GripVertical, ChevronDown, MoreHorizontal } from "lucide-react";
import { cn } from "@/lib/util";
import { fmtOz } from "@/lib/util";
import type { GearRow, Group } from "@/lib/gear";

type Props = {
  groups: Group[];
  reorder?: boolean;
  onEdit: (r: GearRow) => void;
  onDelete: (r: GearRow) => void;
  onMove: (itemId: number, toCategoryId: number, orderedIds: number[]) => void;
  loadoutMode: boolean;
  memberMap: Record<number, "base" | "worn"> | null;
  onToggleMember: (itemId: number, present: boolean, cls?: "base" | "worn") => void;
};

type Container = { id: number; name: string; baseG: number; consumableG: number; wornG: number; itemIds: number[] };

export function GearList(props: Props) {
  const { groups, onEdit, onDelete, onMove, loadoutMode } = props;
  const [containers, setContainers] = useState<Container[]>([]);
  const byId = new Map(groups.flatMap((g) => g.rows).map((r) => [r.id, r]));

  const sig = groups.map((g) => `${g.id}:${g.rows.map((r) => r.id).join(",")}`).join("|");
  useEffect(() => {
    setContainers(
      groups.map((g) => ({
        id: g.id,
        name: g.name,
        baseG: g.baseG,
        consumableG: g.consumableG,
        wornG: g.wornG,
        itemIds: g.rows.map((r) => r.id),
      })),
    );
  }, [sig]);
  // Keep header subtotals fresh when membership changes without a row reshuffle.
  useEffect(() => {
    setContainers((prev) =>
      prev.map((c) => {
        const g = groups.find((x) => x.id === c.id);
        return g ? { ...c, baseG: g.baseG, consumableG: g.consumableG, wornG: g.wornG } : c;
      }),
    );
  }, [groups]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const containerOf = (id: string): number | null => {
    if (id.startsWith("cat:")) return Number(id.slice(4));
    const n = Number(id);
    return containers.find((c) => c.itemIds.includes(n))?.id ?? null;
  };

  const onDragOver = (e: DragOverEvent) => {
    const { active, over } = e;
    if (!over) return;
    const from = containerOf(String(active.id));
    const to = containerOf(String(over.id));
    if (from == null || to == null || from === to) return;
    setContainers((prev) => {
      const activeN = Number(active.id);
      return prev.map((c) => {
        if (c.id === from) return { ...c, itemIds: c.itemIds.filter((i) => i !== activeN) };
        if (c.id === to) {
          const overIdx = over.id.toString().startsWith("cat:")
            ? c.itemIds.length
            : c.itemIds.indexOf(Number(over.id));
          const next = [...c.itemIds];
          next.splice(overIdx < 0 ? next.length : overIdx, 0, activeN);
          return { ...c, itemIds: next };
        }
        return c;
      });
    });
  };

  const onDragEnd = (e: DragEndEvent) => {
    const { active, over } = e;
    if (!over) return;
    const activeN = Number(active.id);
    const to = containerOf(String(over.id));
    if (to == null) return;
    let target = containers.find((c) => c.id === to)!;
    if (!over.id.toString().startsWith("cat:") && active.id !== over.id) {
      const oldIdx = target.itemIds.indexOf(activeN);
      const newIdx = target.itemIds.indexOf(Number(over.id));
      if (oldIdx !== -1 && newIdx !== -1) {
        target = { ...target, itemIds: arrayMove(target.itemIds, oldIdx, newIdx) };
        setContainers((prev) => prev.map((c) => (c.id === to ? target : c)));
      }
    }
    onMove(activeN, to, target.itemIds);
  };

  if (groups.length === 0)
    return (
      <div className="card px-4 py-12 text-center text-sm text-muted">
        Nothing here yet — add a piece of gear to start the log.
      </div>
    );

  return (
    <div className="space-y-3">
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragOver={onDragOver} onDragEnd={onDragEnd}>
        {containers.map((c) => (
          <CategoryCard key={c.id} c={c} byId={byId} {...props} />
        ))}
      </DndContext>
    </div>
  );
}

function CategoryCard({
  c,
  byId,
  onEdit,
  onDelete,
  loadoutMode,
  memberMap,
  reorder,
  onToggleMember,
}: { c: Container; byId: Map<number, GearRow> } & Omit<Props, "groups" | "onMove">) {
  const { setNodeRef } = useDroppable({ id: `cat:${c.id}` });
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const toggle = (id: number) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  return (
    <div className="card">
      <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 border-b bg-panel2/60 px-3 py-2.5 sm:px-4">
        <span className="min-w-0 font-bold break-words">{c.name}</span>
        <span className="readout text-xs sm:text-sm">
          <span className="text-cbase">{fmtOz(c.baseG)} base</span>
          {c.consumableG > 0 && <span className="hidden text-ccons sm:inline"> · {fmtOz(c.consumableG)} cons</span>}
          {c.wornG > 0 && <span className="hidden text-cworn sm:inline"> · {fmtOz(c.wornG)} worn</span>}
        </span>
      </div>
      <div ref={setNodeRef}>
        <SortableContext items={c.itemIds.map(String)} strategy={verticalListSortingStrategy}>
          {c.itemIds.map((id) => {
            const r = byId.get(id);
            if (!r) return null;
            return (
              <div key={id}>
                <SortableRow
                  r={r}
                  reorder={reorder}
                  onEdit={onEdit}
                  onDelete={onDelete}
                  loadoutMode={loadoutMode}
                  member={memberMap ? memberMap[id] : undefined}
                  onToggleMember={onToggleMember}
                  expanded={expanded.has(id)}
                  onToggleExpand={() => toggle(id)}
                />
                {expanded.has(id) && r.components.length > 0 && <KitContents r={r} loadoutMode={loadoutMode} />}
              </div>
            );
          })}
        </SortableContext>
      </div>
    </div>
  );
}

function KitContents({ r, loadoutMode }: { r: GearRow; loadoutMode: boolean }) {
  return (
    <div className={cn("border-b bg-panel2/25 px-4 py-2 last:border-0", loadoutMode ? "pl-14" : "pl-12")}>
      {r.components.map((k) => (
        <div key={k.id} className="flex items-baseline gap-2 py-0.5 text-sm">
          <span className="min-w-0 flex-1 truncate text-muted">
            {k.name}
            {k.quantity > 1 && <span className="text-muted/70"> ×{k.quantity}</span>}
          </span>
          <span className="readout shrink-0 text-xs text-muted">{fmtOz(k.weightG * k.quantity)}</span>
        </div>
      ))}
    </div>
  );
}

function SortableRow({ r, onEdit, onDelete, loadoutMode, member, onToggleMember, expanded, onToggleExpand, reorder }: {
  r: GearRow; onEdit: (r: GearRow) => void; onDelete: (r: GearRow) => void;
  loadoutMode: boolean; member: "base" | "worn" | undefined;
  onToggleMember: Props["onToggleMember"]; expanded: boolean; onToggleExpand: () => void; reorder?: boolean;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: String(r.id) });
  const [menu, setMenu] = useState(false);
  const [sheet, setSheet] = useState(false);
  const included = member !== undefined;
  const canMember = loadoutMode && r.status === "current";
  const worn = (loadoutMode ? member : r.defaultWeightClass) === "worn";
  const close = () => { setMenu(false); setSheet(false); };
  const actions = <>
    <Button variant="ghost" className="w-full justify-start" onClick={() => { close(); onEdit(r); }}>Edit</Button>
    {canMember && <Button variant="ghost" className="w-full justify-start" onClick={() => { close(); onToggleMember(r.id, true, worn ? "base" : "worn"); }}>{worn ? "Mark as packed" : "Mark as worn"}</Button>}
    <Button variant="danger" className="w-full justify-start" onClick={() => { close(); onDelete(r); }}>Delete</Button>
  </>;
  return (
    <div ref={setNodeRef} style={{ transform: CSS.Transform.toString(transform), transition }} className={cn("relative flex items-center gap-2 border-b px-3 py-2 last:border-0 hover:bg-panel2/40 sm:px-4", isDragging && "z-10 bg-panel shadow", canMember && !included && "opacity-45")}>
      <button {...attributes} {...listeners} aria-label={`Drag ${r.name} to reorder`} className={cn("min-h-11 shrink-0 touch-none cursor-grab text-muted", reorder ? "block" : "hidden sm:block")}><GripVertical size={18} /></button>
      {loadoutMode && <span className="grid min-h-11 w-7 shrink-0 place-items-center">{canMember && <input type="checkbox" checked={included} onChange={(e) => onToggleMember(r.id, e.target.checked, r.defaultWeightClass === "worn" ? "worn" : "base")} aria-label={`Include ${r.name}`} className="h-5 w-5 accent-[var(--accent)]" />}</span>}
      {r.isKit && <button onClick={onToggleExpand} aria-label={expanded ? "Collapse kit" : "Expand kit"} aria-expanded={expanded} className="min-h-11 shrink-0 text-muted"><ChevronDown size={16} className={expanded ? "rotate-180" : ""} /></button>}
      <button onClick={() => { if (window.matchMedia("(min-width: 640px)").matches) onEdit(r); else setSheet(true); }} className="min-h-11 min-w-0 flex-1 text-left">
        <span className="block truncate font-semibold">{r.name}{r.quantity > 1 && ` ×${r.quantity}`}</span>
        {(r.isKit || r.status === "retired") && <span className="block text-xs text-muted">{[r.isKit && `Kit · ${r.componentCount} items`, r.status === "retired" && "Retired"].filter(Boolean).join(" · ")}</span>}
      </button>
      {worn && <span className="shrink-0 text-xs font-bold text-signal">Worn</span>}
      <button tabIndex={-1} onClick={() => setSheet(true)} className="shrink-0 text-right text-sm tabular-nums sm:pointer-events-none">{fmtOz(r.weightG == null ? null : r.weightG * r.quantity)}</button>
      <div className="relative hidden sm:block"><IconButton aria-label={`More options for ${r.name}`} aria-expanded={menu} onClick={() => setMenu(!menu)}><MoreHorizontal size={18} /></IconButton>
        {menu && <><button className="fixed inset-0 z-20 cursor-default" aria-label="Close options" onClick={close} /><div role="group" aria-label={`Options for ${r.name}`} className="absolute right-0 top-full z-30 w-44 rounded-xl border bg-panel p-1 shadow-lg" onKeyDown={(e) => { if (e.key === "Escape") close(); }}>{actions}</div></>}
      </div>
      <Sheet open={sheet} onClose={close} title={r.name}><div className="mt-3">{actions}</div></Sheet>
    </div>
  );
}
