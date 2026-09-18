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
import { Card, WornToggle } from "@/components/ui";
import { fmtOz } from "@/lib/util";
import { categoryColor } from "@/lib/categories";
import type { TripGearRow } from "@/lib/trip";

type Handlers = {
  onPacked: (id: number, v: boolean) => void;
  onQty: (id: number, qty: number) => void;
  onClass: (id: number, cls: "base" | "worn") => void;
  onRemove: (id: number) => void;
  onMove: (id: number, toCategory: string, orderedIds: number[]) => void;
};
type Props = { groups: { category: string; rows: TripGearRow[] }[] } & Handlers;

const GRID =
  "grid grid-cols-[1.25rem_1.25rem_minmax(0,1fr)_2.75rem] items-center gap-x-2 sm:grid-cols-[1.25rem_1.25rem_minmax(0,1fr)_3.5rem_5.5rem_4.5rem_1.75rem] sm:gap-x-3";

type Container = { category: string; itemIds: number[] };

export function TripGearList({ groups, ...h }: Props) {
  const [containers, setContainers] = useState<Container[]>([]);
  // Optimistic overlay: reflect packed/qty/class edits instantly and let the
  // server action + page revalidation settle in the background (as the gear tab does).
  const [ov, setOv] = useState<Record<number, Partial<TripGearRow>>>({});
  const patch = (id: number, p: Partial<TripGearRow>) => setOv((o) => ({ ...o, [id]: { ...o[id], ...p } }));
  const byId = new Map(
    groups.flatMap((g) => g.rows).map((r) => [r.id, ov[r.id] ? { ...r, ...ov[r.id] } : r]),
  );
  const handlers: Handlers = {
    ...h,
    onPacked: (id, v) => { patch(id, { packed: v }); h.onPacked(id, v); },
    onQty: (id, qty) => { patch(id, { quantity: qty }); h.onQty(id, qty); },
    onClass: (id, cls) => { patch(id, { weightClass: cls }); h.onClass(id, cls); },
  };

  const sig = groups.map((g) => `${g.category}:${g.rows.map((r) => r.id).join(",")}`).join("|");
  useEffect(() => {
    setContainers(groups.map((g) => ({ category: g.category, itemIds: g.rows.map((r) => r.id) })));
  }, [sig]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const catOf = (id: string): string | null => {
    if (id.startsWith("cat:")) return id.slice(4);
    const n = Number(id);
    return containers.find((c) => c.itemIds.includes(n))?.category ?? null;
  };

  const onDragOver = (e: DragOverEvent) => {
    const { active, over } = e;
    if (!over) return;
    const from = catOf(String(active.id));
    const to = catOf(String(over.id));
    if (from == null || to == null || from === to) return;
    const activeN = Number(active.id);
    setContainers((prev) =>
      prev.map((c) => {
        if (c.category === from) return { ...c, itemIds: c.itemIds.filter((i) => i !== activeN) };
        if (c.category === to) {
          const overIdx = over.id.toString().startsWith("cat:")
            ? c.itemIds.length
            : c.itemIds.indexOf(Number(over.id));
          const next = [...c.itemIds];
          next.splice(overIdx < 0 ? next.length : overIdx, 0, activeN);
          return { ...c, itemIds: next };
        }
        return c;
      }),
    );
  };

  const onDragEnd = (e: DragEndEvent) => {
    const { active, over } = e;
    if (!over) return;
    const activeN = Number(active.id);
    const to = catOf(String(over.id));
    if (to == null) return;
    let target = containers.find((c) => c.category === to)!;
    if (!over.id.toString().startsWith("cat:") && active.id !== over.id) {
      const oldIdx = target.itemIds.indexOf(activeN);
      const newIdx = target.itemIds.indexOf(Number(over.id));
      if (oldIdx !== -1 && newIdx !== -1) {
        target = { ...target, itemIds: arrayMove(target.itemIds, oldIdx, newIdx) };
        setContainers((prev) => prev.map((c) => (c.category === to ? target : c)));
      }
    }
    h.onMove(activeN, to, target.itemIds);
  };

  return (
    <div className="space-y-3">
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragOver={onDragOver} onDragEnd={onDragEnd}>
        {containers.map((c) => (
          <CategoryCard key={c.category} category={c.category} itemIds={c.itemIds} byId={byId} {...handlers} />
        ))}
      </DndContext>
    </div>
  );
}

function CategoryCard({
  category,
  itemIds,
  byId,
  ...h
}: { category: string; itemIds: number[]; byId: Map<number, TripGearRow> } & Handlers) {
  const { setNodeRef } = useDroppable({ id: `cat:${category}` });
  const anchor = `gear-${category.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "")}`;
  return (
    <Card id={anchor} className="scroll-mt-24 overflow-hidden">
      <div className="flex items-center gap-2 border-b bg-panel2/60 px-4 py-2 font-display font-semibold">
        <span className="h-2.5 w-2.5 rounded-sm" style={{ background: categoryColor(category) }} />
        {category}
      </div>
      <div ref={setNodeRef}>
        <SortableContext items={itemIds.map(String)} strategy={verticalListSortingStrategy}>
          {itemIds.map((id) => {
            const r = byId.get(id);
            return r ? <Row key={id} r={r} {...h} /> : null;
          })}
        </SortableContext>
      </div>
    </Card>
  );
}

function Row({ r, onPacked, onQty, onClass, onRemove }: { r: TripGearRow } & Omit<Handlers, "onMove">) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: String(r.id) });
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={`${GRID} border-b px-3 py-2.5 last:border-0 hover:bg-panel2/40 sm:px-4 ${isDragging ? "relative z-10 bg-panel opacity-90 shadow" : ""}`}
    >
      <button {...attributes} {...listeners} aria-label="Drag" className="cursor-grab touch-none text-muted/60 hover:text-muted active:cursor-grabbing">
        ⠿
      </button>
      <input
        type="checkbox"
        checked={r.packed}
        onChange={(e) => onPacked(r.id, e.target.checked)}
        aria-label={`Packed: ${r.name}`}
        className="h-4 w-4 accent-[var(--accent)]"
      />
      <span className={`line-clamp-2 leading-snug sm:block sm:truncate ${r.packed ? "text-muted line-through" : "font-medium"}`}>{r.name}</span>
      <input
        type="number"
        min="1"
        defaultValue={r.quantity}
        onBlur={(e) => Number(e.target.value) !== r.quantity && onQty(r.id, Number(e.target.value))}
        aria-label={`Quantity: ${r.name}`}
        className="hidden w-full rounded-md border bg-panel2 py-1 text-center text-sm outline-none focus:ring-2 focus:ring-accent/25 sm:block [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
      />
      <span className="hidden sm:block"><WornToggle value={r.weightClass} onChange={(cls) => onClass(r.id, cls)} /></span>
      <span className="readout hidden text-right text-sm text-muted sm:block">{fmtOz(r.weightG * r.quantity)}</span>
      <span className="col-span-2 col-start-3 row-start-2 mt-2 flex flex-wrap items-center gap-2 sm:hidden">
        <label className="inline-flex items-center gap-1 text-xs text-muted">
          Qty
          <input
            type="number"
            min="1"
            defaultValue={r.quantity}
            onBlur={(e) => Number(e.target.value) !== r.quantity && onQty(r.id, Number(e.target.value))}
            aria-label={`Quantity: ${r.name}`}
            className="h-11 w-14 rounded-md border bg-panel2 text-center text-base outline-none focus:ring-2 focus:ring-accent/25 [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
          />
        </label>
        <WornToggle value={r.weightClass} onChange={(cls) => onClass(r.id, cls)} />
        <span className="readout text-xs text-muted">{fmtOz(r.weightG * r.quantity)}</span>
      </span>
      <button
        onClick={() => onRemove(r.id)}
        aria-label={`Remove ${r.name}`}
        className="col-start-4 row-start-1 grid h-11 w-11 justify-self-end place-items-center rounded text-muted hover:bg-accent/10 hover:text-accent sm:col-auto sm:row-auto sm:h-auto sm:w-auto sm:p-1"
      >
        ✕
      </button>
    </div>
  );
}
