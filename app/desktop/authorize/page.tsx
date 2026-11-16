import { randomBytes } from "node:crypto";
import { redirect } from "next/navigation";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { hashToken } from "@/lib/sync/device";
import { parseAuthorization, type AuthorizationQuery } from "@/lib/sync/authorize";
import { PageHeader, PageBody } from "@/components/page-header";
import { Button, Card } from "@/components/ui";
export const dynamic = "force-dynamic";
export const metadata = { title: "Allow Strider Desktop" };
export default async function AuthorizePage({ searchParams }: { searchParams: Promise<AuthorizationQuery> }) {
  const query = await searchParams;
  const authorization = parseAuthorization(query);
  async function decide(form: FormData) {
    "use server";
    const userId = await getCurrentUserId();
    const params = parseAuthorization(query);
    if (!params) return;
    const destination = new URL(params.redirectUri);
    destination.searchParams.set("state", params.state);
    if (form.get("decision") === "allow") {
      const code = randomBytes(32).toString("base64url");
      await db.insert(schema.deviceAuthCode).values({ codeHash: hashToken(code), userId, codeChallenge: params.challenge, deviceName: params.deviceName, expiresAt: new Date(Date.now() + 60_000) });
      destination.searchParams.set("code", code);
    } else destination.searchParams.set("error", "access_denied");
    redirect(destination.toString());
  }
  return <>
    <PageHeader title="Allow Strider Desktop" />
    <PageBody>
      <Card className="p-4 sm:p-5">
        {authorization ? <form action={decide} className="space-y-4">
          <p className="text-sm">Allow Strider on {authorization.deviceName} to access your workspace?</p>
          <div className="flex flex-wrap gap-3">
            <Button type="submit" name="decision" value="allow">Allow</Button>
            <Button type="submit" name="decision" value="cancel" variant="ghost">Cancel</Button>
          </div>
        </form> : <p className="text-sm text-muted">Invalid desktop authorization request.</p>}
      </Card>
    </PageBody>
  </>;
}
