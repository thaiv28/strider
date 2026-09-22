import { eq, asc } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { OTHER_CATEGORY_ID, OTHER_CATEGORY_NAME } from "@/lib/categories";
import { auth } from "@/auth";

export type WClass = "base" | "worn" | "consumable";

export { OTHER_CATEGORY_ID, OTHER_CATEGORY_NAME };

export type GearComponent = { id: number; name: string; weightG: number; quantity: number; notes: string | null };

export type GearRow = {
  id: number;
  name: string;
  weightG: number | null; // per-unit
  priceCents: number | null; // per-unit
  quantity: number;
  defaultWeightClass: WClass;
  status: "current" | "retired" | "wishlist";
  isKit: boolean;
  productUrl: string | null;
  notes: string | null;
  categoryId: number | null;
  slotName: string | null;
  components: GearComponent[];
  componentCount: number;
  replaces: number[]; // current gear-item ids this wishlist item would retire
};

export type CategoryOption = { id: number; name: string };
export type Group = {
  id: number;
  name: string;
  rows: GearRow[];
  baseG: number;
  consumableG: number;
  wornG: number;
};

export type LoadoutLite = { id: number; name: string; isDefault: boolean };
export type Membership = Record<number, Record<number, "base" | "worn">>;
// wishlistItemId → loadoutId → forced included/excluded (overrides the assumption).
export type WishlistOverrides = Record<number, Record<number, boolean>>;

export async function getLoadoutData(
  userId: number,
): Promise<{ loadouts: LoadoutLite[]; membership: Membership; wishlistOverrides: WishlistOverrides }> {
  const loadouts = await db
    .select({ id: schema.loadout.id, name: schema.loadout.name, isDefault: schema.loadout.isDefault })
    .from(schema.loadout)
    .where(eq(schema.loadout.userId, userId))
    .orderBy(asc(schema.loadout.id));
  const items = await db
    .select({
      loadoutId: schema.loadoutItem.loadoutId,
      gearItemId: schema.loadoutItem.gearItemId,
      weightClass: schema.loadoutItem.weightClass,
    })
    .from(schema.loadoutItem)
    .innerJoin(schema.loadout, eq(schema.loadoutItem.loadoutId, schema.loadout.id))
    .where(eq(schema.loadout.userId, userId));
  const overrides = await db
    .select({
      wishlistItemId: schema.wishlistLoadout.wishlistItemId,
      loadoutId: schema.wishlistLoadout.loadoutId,
      included: schema.wishlistLoadout.included,
    })
    .from(schema.wishlistLoadout)
    .innerJoin(schema.loadout, eq(schema.wishlistLoadout.loadoutId, schema.loadout.id))
    .where(eq(schema.loadout.userId, userId));
  const membership: Membership = {};
  for (const l of loadouts) membership[l.id] = {};
  for (const it of items) {
    if (membership[it.loadoutId]) membership[it.loadoutId][it.gearItemId] = it.weightClass;
  }
  const wishlistOverrides: WishlistOverrides = {};
  for (const o of overrides) {
    (wishlistOverrides[o.wishlistItemId] ??= {})[o.loadoutId] = o.included;
  }
  loadouts.sort((a, b) => Number(b.isDefault) - Number(a.isDefault) || a.id - b.id);
  return { loadouts, membership, wishlistOverrides };
}

// Base-weight breakdown for the default loadout (falls back to all current
// base gear if no loadout exists). Used by the Basecamp base bar.
export type SliceItem = { name: string; g: number; quantity: number };
export type CatSlice = { name: string; g: number; items: SliceItem[] };

export async function getDefaultBaseBreakdown(
  userId: number,
): Promise<{ baseG: number; byCategory: CatSlice[] }> {
  const [view, { loadouts, membership }] = await Promise.all([
    getGearView(userId),
    getLoadoutData(userId),
  ]);
  const def = loadouts.find((l) => l.isDefault) ?? loadouts[0];
  if (!def) return { baseG: view.totals.baseG, byCategory: view.byCategory };
  const m = membership[def.id] ?? {};
  const byCat = new Map<string, CatSlice>();
  for (const g of view.groups)
    for (const r of g.rows)
      if (r.status === "current" && m[r.id] === "base") {
        const cur = byCat.get(g.name) ?? { name: g.name, g: 0, items: [] };
        const w = (r.weightG ?? 0) * r.quantity;
        cur.g += w;
        cur.items.push({ name: r.name, g: w, quantity: r.quantity });
        byCat.set(g.name, cur);
      }
  const byCategory = [...byCat.values()].filter((c) => c.g > 0);
  return { baseG: byCategory.reduce((s, c) => s + c.g, 0), byCategory };
}

export async function getCurrentUserId(): Promise<number> {
  const session = await auth();
  const userId = Number(session?.user?.id);
  if (!Number.isInteger(userId) || userId <= 0) throw new Error("Unauthorized");
  return userId;
}

export async function getGearView(userId: number) {
  const cats = await db
    .select()
    .from(schema.gearCategory)
    .where(eq(schema.gearCategory.userId, userId))
    .orderBy(asc(schema.gearCategory.sortOrder));
  const items = await db
    .select()
    .from(schema.gearItem)
    .where(eq(schema.gearItem.userId, userId))
    .orderBy(asc(schema.gearItem.sortOrder), asc(schema.gearItem.name));
  const comps = await db
    .select({
      id: schema.gearComponent.id,
      gearItemId: schema.gearComponent.gearItemId,
      name: schema.gearComponent.name,
      weightG: schema.gearComponent.weightG,
      quantity: schema.gearComponent.quantity,
      notes: schema.gearComponent.notes,
    })
    .from(schema.gearComponent)
    .innerJoin(schema.gearItem, eq(schema.gearComponent.gearItemId, schema.gearItem.id))
    .where(eq(schema.gearItem.userId, userId))
    .orderBy(asc(schema.gearComponent.id));
  const repls = await db
    .select({
      wishlistItemId: schema.wishlistReplacement.wishlistItemId,
      replacesItemId: schema.wishlistReplacement.replacesItemId,
    })
    .from(schema.wishlistReplacement)
    .innerJoin(schema.gearItem, eq(schema.wishlistReplacement.wishlistItemId, schema.gearItem.id))
    .where(eq(schema.gearItem.userId, userId));

  const compsByItem = new Map<number, GearComponent[]>();
  for (const c of comps) {
    if (!compsByItem.has(c.gearItemId)) compsByItem.set(c.gearItemId, []);
    compsByItem.get(c.gearItemId)!.push({ id: c.id, name: c.name, weightG: c.weightG, quantity: c.quantity, notes: c.notes });
  }
  const replByItem = new Map<number, number[]>();
  for (const r of repls) {
    if (!replByItem.has(r.wishlistItemId)) replByItem.set(r.wishlistItemId, []);
    replByItem.get(r.wishlistItemId)!.push(r.replacesItemId);
  }
  const catById = new Map(cats.map((c) => [c.id, c]));

  const rows: GearRow[] = items.map((i) => {
    const components = compsByItem.get(i.id) ?? [];
    return {
      id: i.id,
      name: i.name,
      weightG: i.weightG,
      priceCents: i.priceCents,
      quantity: i.quantity,
      defaultWeightClass: i.defaultWeightClass,
      status: i.status,
      isKit: i.isKit,
      productUrl: i.productUrl,
      notes: i.notes,
      categoryId: i.categoryId,
      slotName: i.categoryId ? (catById.get(i.categoryId)?.name ?? null) : null,
      components,
      componentCount: components.length,
      replaces: replByItem.get(i.id) ?? [],
    };
  });

  const subtotal = (categoryId: number, cls: WClass) =>
    rows
      .filter((r) => r.categoryId === categoryId && r.status === "current" && r.defaultWeightClass === cls)
      .reduce((s, r) => s + (r.weightG ?? 0) * r.quantity, 0);

  const groups: Group[] = cats
    .map((c) => ({
      id: c.id,
      name: c.name,
      rows: rows.filter((r) => r.categoryId === c.id),
      baseG: subtotal(c.id, "base"),
      consumableG: subtotal(c.id, "consumable"),
      wornG: subtotal(c.id, "worn"),
    }))
    .filter((g) => g.rows.length > 0);

  // Uncategorized items land in a synthetic "Other" group rather than vanishing.
  const otherRows = rows.filter((r) => r.categoryId == null);
  if (otherRows.length) {
    const otherSubtotal = (cls: WClass) =>
      otherRows
        .filter((r) => r.status === "current" && r.defaultWeightClass === cls)
        .reduce((s, r) => s + (r.weightG ?? 0) * r.quantity, 0);
    groups.push({
      id: OTHER_CATEGORY_ID,
      name: OTHER_CATEGORY_NAME,
      rows: otherRows,
      baseG: otherSubtotal("base"),
      consumableG: otherSubtotal("consumable"),
      wornG: otherSubtotal("worn"),
    });
  }

  const wishlist = rows.filter((r) => r.status === "wishlist");

  const current = rows.filter((r) => r.status === "current");
  const classTotal = (cls: WClass) =>
    current.filter((r) => r.defaultWeightClass === cls).reduce((s, r) => s + (r.weightG ?? 0) * r.quantity, 0);

  const byCategory: CatSlice[] = groups
    .map((g) => ({
      name: g.name,
      g: g.baseG,
      items: g.rows
        .filter((r) => r.status === "current" && r.defaultWeightClass === "base")
        .map((r) => ({ name: r.name, g: (r.weightG ?? 0) * r.quantity, quantity: r.quantity })),
    }))
    .filter((c) => c.g > 0);

  const categoryOptions: CategoryOption[] = cats.map((c) => ({ id: c.id, name: c.name }));

  return {
    groups,
    wishlist,
    categoryOptions,
    byCategory,
    totals: {
      baseG: classTotal("base"),
      wornG: classTotal("worn"),
      consumableG: classTotal("consumable"),
    },
  };
}
