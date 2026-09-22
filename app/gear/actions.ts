"use server";

import { revalidatePath } from "next/cache";
import { eq, and, isNull } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId, OTHER_CATEGORY_ID } from "@/lib/gear";
import { ozToG } from "@/lib/util";
import {
  requireOwnedCategory,
  requireOwnedGearItem,
  requireOwnedGearItems,
  requireOwnedLoadout,
} from "@/lib/authorization";

type WClass = "base" | "worn" | "consumable";
type Status = "current" | "retired" | "wishlist";

type Component = { name: string; weightG: number; quantity: number; notes: string | null };

// Components arrive as JSON (weights already in grams). Ignored unless isKit.
function parseComponents(fd: FormData): Component[] {
  try {
    const raw = JSON.parse(String(fd.get("components") ?? "[]"));
    if (!Array.isArray(raw)) return [];
    return raw
      .map((c) => ({
        name: String(c?.name ?? "").trim(),
        weightG: Math.max(0, Math.round(Number(c?.weightG) || 0)),
        quantity: Math.max(1, Math.round(Number(c?.quantity) || 1)),
        notes: c?.notes ? String(c.notes).trim() || null : null,
      }))
      .filter((c) => c.name);
  } catch {
    return [];
  }
}

function parseForm(fd: FormData) {
  const oz = fd.get("weightOz");
  const ozNum = oz != null && String(oz).trim() !== "" ? Number(oz) : null;
  const price = fd.get("priceUsd");
  const priceNum = price != null && String(price).trim() !== "" ? Number(price) : null;
  const categoryId = fd.get("categoryId");
  const qty = Number(fd.get("quantity"));
  const isKit = fd.get("isKit") === "on" || fd.get("isKit") === "true";
  const components = isKit ? parseComponents(fd) : [];
  // A kit's per-unit weight is the sum of its components; the manual field is ignored.
  const kitWeightG = components.reduce((s, c) => s + c.weightG * c.quantity, 0);
  return {
    name: String(fd.get("name") ?? "").trim(),
    weightG: isKit ? kitWeightG : ozNum != null && Number.isFinite(ozNum) ? ozToG(ozNum) : null,
    priceCents: priceNum != null && Number.isFinite(priceNum) ? Math.max(0, Math.round(priceNum * 100)) : null,
    quantity: Number.isFinite(qty) && qty >= 1 ? Math.round(qty) : 1,
    defaultWeightClass: (String(fd.get("defaultWeightClass") ?? "base") as WClass),
    status: (String(fd.get("status") ?? "current") as Status),
    categoryId: categoryId && String(categoryId) !== "" ? Number(categoryId) : null,
    productUrl: String(fd.get("productUrl") ?? "").trim() || null,
    notes: String(fd.get("notes") ?? "").trim() || null,
    isKit,
    components,
  };
}

type Tx = Parameters<Parameters<typeof db.transaction>[0]>[0];

async function writeComponents(tx: Tx, gearItemId: number, isKit: boolean, components: Component[]) {
  await tx.delete(schema.gearComponent).where(eq(schema.gearComponent.gearItemId, gearItemId));
  if (isKit && components.length)
    await tx.insert(schema.gearComponent).values(components.map((c) => ({ gearItemId, ...c })));
}

export async function createGear(fd: FormData) {
  const userId = await getCurrentUserId();
  const v = parseForm(fd);
  if (!v.name) throw new Error("Name is required");
  await requireOwnedCategory(v.categoryId, userId);
  await db.transaction(async (tx) => {
    const [row] = await tx
      .insert(schema.gearItem)
      .values({
        userId,
        name: v.name,
        weightG: v.weightG,
        priceCents: v.priceCents,
        quantity: v.quantity,
        defaultWeightClass: v.defaultWeightClass,
        status: v.status,
        categoryId: v.categoryId,
        isKit: v.isKit,
        productUrl: v.productUrl,
        notes: v.notes,
      })
      .returning({ id: schema.gearItem.id });
    await writeComponents(tx, row.id, v.isKit, v.components);
  });
  revalidatePath("/gear");
}

export async function updateGear(id: number, fd: FormData) {
  const userId = await requireOwnedGearItem(id);
  const v = parseForm(fd);
  if (!v.name) throw new Error("Name is required");
  await requireOwnedCategory(v.categoryId, userId);
  await db.transaction(async (tx) => {
    await tx
      .update(schema.gearItem)
      .set({
        name: v.name,
        weightG: v.weightG,
        priceCents: v.priceCents,
        quantity: v.quantity,
        defaultWeightClass: v.defaultWeightClass,
        status: v.status,
        categoryId: v.categoryId,
        isKit: v.isKit,
        productUrl: v.productUrl,
        notes: v.notes,
        updatedAt: new Date(),
      })
      .where(eq(schema.gearItem.id, id));
    await writeComponents(tx, id, v.isKit, v.components);
  });
  revalidatePath("/gear");
}

export async function deleteGear(id: number) {
  await requireOwnedGearItem(id);
  await db.delete(schema.gearItem).where(eq(schema.gearItem.id, id));
  revalidatePath("/gear");
}

// Replace the full set of current items a wishlist item would retire.
export async function setWishlistReplacements(wishlistItemId: number, replacesIds: number[]) {
  const ids = [...new Set(replacesIds)].filter((n) => Number.isFinite(n) && n !== wishlistItemId);
  const userId = await requireOwnedGearItem(wishlistItemId);
  await requireOwnedGearItems(ids, userId);
  await db.transaction(async (tx) => {
    await tx
      .delete(schema.wishlistReplacement)
      .where(eq(schema.wishlistReplacement.wishlistItemId, wishlistItemId));
    if (ids.length)
      await tx
        .insert(schema.wishlistReplacement)
        .values(ids.map((replacesItemId) => ({ wishlistItemId, replacesItemId })));
  });
  revalidatePath("/gear");
}

// Force a wishlist item in/out of a loadout's projection, or clear the override
// (included === null) to fall back to the assumed membership.
export async function setWishlistLoadout(
  wishlistItemId: number,
  loadoutId: number,
  included: boolean | null,
) {
  const userId = await requireOwnedGearItem(wishlistItemId);
  await requireOwnedLoadout(loadoutId, userId);
  if (included === null) {
    await db
      .delete(schema.wishlistLoadout)
      .where(
        and(
          eq(schema.wishlistLoadout.wishlistItemId, wishlistItemId),
          eq(schema.wishlistLoadout.loadoutId, loadoutId),
        ),
      );
  } else {
    await db
      .insert(schema.wishlistLoadout)
      .values({ wishlistItemId, loadoutId, included })
      .onConflictDoUpdate({
        target: [schema.wishlistLoadout.wishlistItemId, schema.wishlistLoadout.loadoutId],
        set: { included },
      });
  }
  revalidatePath("/gear");
}

export async function createLoadout(name: string): Promise<number> {
  const userId = await getCurrentUserId();
  const [l] = await db
    .insert(schema.loadout)
    .values({ userId, name: name.trim() || "New Loadout" })
    .returning({ id: schema.loadout.id });
  revalidatePath("/gear");
  return l.id;
}

export async function renameLoadout(id: number, name: string) {
  await requireOwnedLoadout(id);
  await db.update(schema.loadout).set({ name: name.trim() || "Loadout" }).where(eq(schema.loadout.id, id));
  revalidatePath("/gear");
}

export async function deleteLoadout(id: number) {
  await requireOwnedLoadout(id);
  await db.delete(schema.loadout).where(eq(schema.loadout.id, id));
  revalidatePath("/gear");
}

export async function setDefaultLoadout(id: number) {
  const userId = await requireOwnedLoadout(id);
  await db.transaction(async (tx) => {
    await tx.update(schema.loadout).set({ isDefault: false }).where(eq(schema.loadout.userId, userId));
    await tx.update(schema.loadout).set({ isDefault: true }).where(eq(schema.loadout.id, id));
  });
  revalidatePath("/gear");
}

// Add/remove a gear item from a loadout, or change its worn/packed class.
export async function setLoadoutMember(
  loadoutId: number,
  gearItemId: number,
  present: boolean,
  cls: "base" | "worn" = "base",
) {
  const userId = await requireOwnedLoadout(loadoutId);
  await requireOwnedGearItem(gearItemId, userId);
  if (present) {
    await db
      .insert(schema.loadoutItem)
      .values({ loadoutId, gearItemId, weightClass: cls })
      .onConflictDoUpdate({
        target: [schema.loadoutItem.loadoutId, schema.loadoutItem.gearItemId],
        set: { weightClass: cls },
      });
  } else {
    await db
      .delete(schema.loadoutItem)
      .where(and(eq(schema.loadoutItem.loadoutId, loadoutId), eq(schema.loadoutItem.gearItemId, gearItemId)));
  }
  revalidatePath("/gear");
}

// Reorder within a category (cosmetic) or move to another (changes category).
// `orderedIds` is the desired order of the target category's visible items;
// any other items in that category keep their relative order after them.
export async function moveGear(itemId: number, toCategoryId: number, orderedIds: number[]) {
  const userId = await requireOwnedGearItem(itemId);
  const targetCat = toCategoryId === OTHER_CATEGORY_ID ? null : toCategoryId;
  await requireOwnedCategory(targetCat, userId);
  await requireOwnedGearItems(orderedIds, userId);
  await db.transaction(async (tx) => {
    await tx
      .update(schema.gearItem)
      .set({ categoryId: targetCat })
      .where(eq(schema.gearItem.id, itemId));

    const all = await tx
      .select({ id: schema.gearItem.id })
      .from(schema.gearItem)
      .where(
        and(
          eq(schema.gearItem.userId, userId),
          targetCat === null
            ? isNull(schema.gearItem.categoryId)
            : eq(schema.gearItem.categoryId, targetCat),
        ),
      )
      .orderBy(schema.gearItem.sortOrder);

    const seen = new Set(orderedIds);
    const finalOrder = [...orderedIds, ...all.map((a) => a.id).filter((id) => !seen.has(id))];
    for (let i = 0; i < finalOrder.length; i++) {
      await tx.update(schema.gearItem).set({ sortOrder: i }).where(eq(schema.gearItem.id, finalOrder[i]));
    }
  });
  revalidatePath("/gear");
  revalidatePath("/basecamp");
}
