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
import { Badge, WornToggle } from "@/components/ui";
import { cn } from "@/lib/util";
import { fmtOz } from "@/lib/util";
import type { GearRow, Group } from "@/lib/gear";

type Props = {
  groups: Group[];
  onEdit: (r: GearRow) => void;
  onDelete: (r: GearRow) => void;
  onMove: (itemId: number, toCategoryId: number, orderedIds: number[]) => void;
  loadoutMode: boolean;
  memberMap: Record<number, "base" | "worn"> | null;
  onToggleMember: (itemId: number, present: boolean, cls?: "base" | "worn") => void;
};

const GRID_BASE = "grid grid-cols-[1.25rem_1.25rem_minmax(0,1fr)_2.75rem] items-center gap-x-2 sm:grid-cols-[1.25rem_1.25rem_minmax(0,1fr)_5rem_4.5rem_1.75rem]";
const GRID_LOADOUT =
  "grid grid-cols-[1.5rem_1.25rem_1.25rem_minmax(0,1fr)_2.75rem] items-center gap-x-2 sm:grid-cols-[1.5rem_1.25rem_1.25rem_minmax(0,1fr)_6.5rem_4.5rem_1.75rem]";

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
    <div className="card overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 border-b bg-panel2/60 px-3 py-2.5 sm:px-4">
        <span className="min-w-0 font-display font-semibold break-words">{c.name}</span>
        <span className="readout text-xs sm:text-sm">
          <span className="text-cbase">{fmtOz(c.baseG)} base</span>
          {c.consumableG > 0 && <span className="text-ccons"> · {fmtOz(c.consumableG)} cons</span>}
          {c.wornG > 0 && <span className="text-cworn"> · {fmtOz(c.wornG)} worn</span>}
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

function SortableRow({
  r,
  onEdit,
  onDelete,
  loadoutMode,
  member,
  onToggleMember,
  expanded,
  onToggleExpand,
}: {
  r: GearRow;
  onEdit: (r: GearRow) => void;
  onDelete: (r: GearRow) => void;
  loadoutMode: boolean;
  member: "base" | "worn" | undefined;
  onToggleMember: (itemId: number, present: boolean, cls?: "base" | "worn") => void;
  expanded: boolean;
  onToggleExpand: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: String(r.id) });
  const included = member !== undefined;
  const canMember = loadoutMode && r.status === "current";

  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={cn(
        loadoutMode ? GRID_LOADOUT : GRID_BASE,
        "border-b px-3 py-2.5 last:border-0 hover:bg-panel2/40 sm:px-4 sm:py-2",
        isDragging && "relative z-10 bg-panel opacity-90 shadow",
        canMember && !included && "opacity-45",
      )}
    >
      {loadoutMode &&
        (canMember ? (
          <input
            type="checkbox"
            checked={included}
            onChange={(e) => onToggleMember(r.id, e.target.checked, r.defaultWeightClass === "worn" ? "worn" : "base")}
            aria-label={`Include ${r.name}`}
            className="h-4 w-4 accent-[var(--accent)]"
          />
        ) : (
          <span />
        ))}
      <button
        {...attributes}
        {...listeners}
        aria-label="Drag to reorder"
        className="cursor-grab touch-none text-muted/60 hover:text-muted active:cursor-grabbing"
      >
        ⠿
      </button>
      {r.isKit && r.componentCount > 0 ? (
        <button
          onClick={onToggleExpand}
          aria-label={expanded ? "Collapse kit" : "Expand kit"}
          title={expanded ? "Collapse contents" : "Show contents"}
          className="text-cworn transition hover:text-accent"
        >
          {expanded ? "▤" : "▣"}
        </button>
      ) : (
        <span className="text-cworn">{r.isKit ? "▣" : ""}</span>
      )}
      <button
        onClick={() => onEdit(r)}
        className="flex min-w-0 items-baseline gap-1.5 py-1 text-left transition hover:text-accent sm:py-0.5"
      >
        <span className="line-clamp-2 font-medium leading-snug sm:block sm:truncate">{r.name}</span>
        {r.quantity > 1 && <span className="shrink-0 text-sm text-muted">×{r.quantity}</span>}
        {r.isKit && r.componentCount > 0 && (
          <span className="shrink-0 text-xs text-muted">· {r.componentCount}</span>
        )}
      </button>
      <span className="hidden text-right sm:block">
        {canMember && included ? (
          <WornToggle value={member!} onChange={(cls) => onToggleMember(r.id, true, cls)} />
        ) : !loadoutMode && r.defaultWeightClass !== "base" ? (
          <Badge tone={r.defaultWeightClass}>{r.defaultWeightClass === "consumable" ? "cons" : r.defaultWeightClass}</Badge>
        ) : null}
      </span>
      <span className="readout hidden text-right text-sm text-muted sm:block">
        {fmtOz(r.weightG == null ? null : r.weightG * r.quantity)}
      </span>
      <span className={cn("col-span-2 col-start-3 row-start-2 mt-1 flex min-w-0 flex-wrap items-center gap-2 pb-0.5 text-xs text-muted sm:hidden", loadoutMode && "col-start-4")}>
        <span className="readout">{fmtOz(r.weightG == null ? null : r.weightG * r.quantity)}</span>
        {canMember && included ? (
          <WornToggle value={member!} onChange={(cls) => onToggleMember(r.id, true, cls)} />
        ) : !loadoutMode && r.defaultWeightClass !== "base" ? (
          <Badge tone={r.defaultWeightClass}>{r.defaultWeightClass === "consumable" ? "cons" : r.defaultWeightClass}</Badge>
        ) : null}
      </span>
      <DeleteBtn r={r} onDelete={onDelete} loadoutMode={loadoutMode} />
    </div>
  );
}

function DeleteBtn({ r, onDelete, loadoutMode }: { r: GearRow; onDelete: (r: GearRow) => void; loadoutMode: boolean }) {
  return (
    <button
      onClick={() => onDelete(r)}
      aria-label={`Delete ${r.name}`}
      className={cn(
        "col-start-4 row-start-1 grid h-11 w-11 justify-self-end place-items-center rounded text-muted transition hover:bg-accent/10 hover:text-accent sm:col-auto sm:row-auto sm:h-auto sm:w-auto sm:p-1",
        loadoutMode && "col-start-5 sm:col-auto",
      )}
    >
      ✕
    </button>
  );
}
