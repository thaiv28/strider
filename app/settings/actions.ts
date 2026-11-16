"use server";

import { revalidatePath } from "next/cache";
import { and, eq, isNull } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { requireOwnedCalendarFeed } from "@/lib/authorization";

const s = (v: number | null) => (v == null ? null : String(v));

export type EnergyInput = {
  bmr: number;
  calPerMile: number;
  ftPerMile: number;
  sex: string | null;
  weightLb: number | null;
  heightIn: number | null;
  ageYears: number | null;
};

export async function updateEnergyParams(v: EnergyInput) {
  const userId = await getCurrentUserId();
  const row = {
    userId,
    bmr: Math.max(0, Math.round(v.bmr)),
    calPerEnergyMile: Math.max(0, Math.round(v.calPerMile)),
    ftPerEnergyMile: Math.max(1, Math.round(v.ftPerMile)),
    sex: v.sex,
    weightLb: s(v.weightLb),
    heightIn: s(v.heightIn),
    ageYears: v.ageYears,
  };
  await db
    .insert(schema.energyParams)
    .values(row)
    .onConflictDoUpdate({ target: schema.energyParams.userId, set: row });
  revalidatePath("/settings");
  revalidatePath("/trips");
}

export async function updateReportSettings(v: { template: string; packingDefault: string }) {
  const userId = await getCurrentUserId();
  const row = { userId, template: v.template, packingDefault: v.packingDefault };
  await db
    .insert(schema.reportSettings)
    .values(row)
    .onConflictDoUpdate({ target: schema.reportSettings.userId, set: row });
  revalidatePath("/settings");
}

async function bumpCalendar() {
  revalidatePath("/settings");
  revalidatePath("/calendar");
}

export async function addCalendarFeed(v: { label: string; color: string; url: string }) {
  const userId = await getCurrentUserId();
  await db.insert(schema.calendarFeed).values({ userId, label: v.label.trim() || "Calendar", color: v.color, url: v.url.trim() });
  await bumpCalendar();
}

export async function updateCalendarFeed(id: number, v: { label: string; color: string; url: string }) {
  await requireOwnedCalendarFeed(id);
  await db
    .update(schema.calendarFeed)
    .set({ label: v.label.trim() || "Calendar", color: v.color, url: v.url.trim() })
    .where(eq(schema.calendarFeed.id, id));
  await bumpCalendar();
}

export async function deleteCalendarFeed(id: number) {
  await requireOwnedCalendarFeed(id);
  await db.delete(schema.calendarFeed).where(eq(schema.calendarFeed.id, id));
  await bumpCalendar();
}

export async function listDevices() {
  const userId = await getCurrentUserId();
  return db.select({ id: schema.deviceSession.id, deviceName: schema.deviceSession.deviceName, createdAt: schema.deviceSession.createdAt, lastUsedAt: schema.deviceSession.lastUsedAt })
    .from(schema.deviceSession).where(and(eq(schema.deviceSession.userId, userId), isNull(schema.deviceSession.revokedAt))).orderBy(schema.deviceSession.createdAt);
}

export async function revokeDevice(id: number) {
  const userId = await getCurrentUserId();
  await db.update(schema.deviceSession).set({ revokedAt: new Date() }).where(and(eq(schema.deviceSession.id, id), eq(schema.deviceSession.userId, userId)));
  revalidatePath("/settings");
}
