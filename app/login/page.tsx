import { redirect } from "next/navigation";

export default async function LoginPage({ searchParams }: {
  searchParams: Promise<{ callbackUrl?: string; error?: string; test?: string }>;
}) {
  const params = await searchParams;
  const query = new URLSearchParams();
  for (const key of ["callbackUrl", "error", "test"] as const) {
    if (params[key] !== undefined) query.set(key, params[key]);
  }
  redirect(query.size ? `/?${query}` : "/");
}
