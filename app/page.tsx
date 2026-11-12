import { LandingPage } from "@/components/landing/landing-page";
import { auth } from "@/auth";
import { redirect } from "next/navigation";

export const metadata = {
  title: { absolute: "Strider" },
  description: "Plan your backpacking trip in one place: route, gear and food.",
};

export default async function Page({ searchParams }: {
  searchParams: Promise<{ callbackUrl?: string; error?: string; test?: string }>;
}) {
  const { callbackUrl, error, test } = await searchParams;
  const returnTo = callbackUrl?.startsWith("/") && !callbackUrl.startsWith("//") && !callbackUrl.includes("\\") ? callbackUrl : "/basecamp";
  const session = await auth();
  if (session && callbackUrl !== undefined) redirect(returnTo);
  return <LandingPage signedIn={!!session} callbackUrl={returnTo} error={!!error} test={test === "1"} />;
}
