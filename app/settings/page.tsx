import { eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { EnergyModel } from "@/components/settings/energy-model";
import { ReportTemplate } from "@/components/settings/report-template";
import { CalendarFeeds } from "@/components/settings/calendar-feeds";
import { DEFAULT_TEMPLATE, DEFAULT_PACKING } from "@/lib/report";
import { AppearanceSetting } from "@/components/appearance-setting";
import { PageHeader, PageBody } from "@/components/page-header";
import { Button } from "@/components/ui";
import { logout } from "@/app/login/actions";

export const dynamic = "force-dynamic";
export const metadata = { title: "Settings" };

const num = (s: string | null) => (s == null ? null : Number(s));

export default async function SettingsPage() {
  const userId = await getCurrentUserId();
  const [ep] = await db.select().from(schema.energyParams).where(eq(schema.energyParams.userId, userId));
  const [rs] = await db.select().from(schema.reportSettings).where(eq(schema.reportSettings.userId, userId));
  const feeds = await db.select().from(schema.calendarFeed).where(eq(schema.calendarFeed.userId, userId)).orderBy(schema.calendarFeed.sortOrder);

  return (
    <>
      <PageHeader title="Settings" actions={<form action={logout}><Button variant="header" type="submit">Log out</Button></form>} />
      <PageBody className="max-w-2xl space-y-6">
        <AppearanceSetting />
        <EnergyModel
          params={{
            bmr: ep?.bmr ?? 1735,
            calPerMile: ep?.calPerEnergyMile ?? 200,
            ftPerMile: ep?.ftPerEnergyMile ?? 625,
            sex: ep?.sex ?? null,
            weightLb: num(ep?.weightLb ?? null),
            heightIn: num(ep?.heightIn ?? null),
            ageYears: ep?.ageYears ?? null,
          }}
        />

        <ReportTemplate
          template={rs?.template ?? DEFAULT_TEMPLATE}
          packingDefault={rs?.packingDefault ?? DEFAULT_PACKING}
        />

        <CalendarFeeds feeds={feeds.map((f) => ({ id: f.id, label: f.label, color: f.color, url: f.url }))} />
      </PageBody>
    </>
  );
}
