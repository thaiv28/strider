"use server";

import { revalidatePath } from "next/cache";
import { eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";

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
  await db
    .update(schema.calendarFeed)
    .set({ label: v.label.trim() || "Calendar", color: v.color, url: v.url.trim() })
    .where(eq(schema.calendarFeed.id, id));
  await bumpCalendar();
}

export async function deleteCalendarFeed(id: number) {
  await db.delete(schema.calendarFeed).where(eq(schema.calendarFeed.id, id));
  await bumpCalendar();
}
