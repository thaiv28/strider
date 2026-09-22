import { auth } from "@/auth";
import { loginForE2e, loginWithGoogle } from "./actions";
import { redirect } from "next/navigation";

export const metadata = { title: "Sign in" };

export default async function LoginPage({
  searchParams,
}: {
  searchParams: Promise<{ error?: string; test?: string }>;
}) {
  if (await auth()) redirect("/basecamp");
  const { error, test } = await searchParams;
  return (
    <main className="grid min-h-screen place-items-center px-4 py-12">
      <section className="card w-full max-w-sm p-6 sm:p-8">
        <p className="eyebrow">Your trail workspace</p>
        <h1 className="mt-2 font-display text-3xl font-bold">STRIDER</h1>
        <p className="mt-3 text-sm text-muted">Sign in to plan trips, gear, food, and trail logistics in your private workspace.</p>
        <form action={loginWithGoogle} className="mt-6">
          <button className="w-full rounded-md bg-accent px-4 py-3 font-semibold text-accentink" type="submit">
            Continue with Google
          </button>
        </form>
        {error ? <p className="mt-4 text-sm text-red-700 dark:text-red-300">Google sign-in could not be completed.</p> : null}
        {test === "1" ? (
          <form action={loginForE2e} className="mt-6 grid gap-3 border-t pt-5">
            <label className="grid gap-1.5 text-sm font-semibold">
              Test account
              <input className="rounded-md border bg-canvas px-3 py-2 text-ink" defaultValue="e2e@strider.invalid" name="email" required type="email" />
            </label>
            <label className="grid gap-1.5 text-sm font-semibold">
              Test password
              <input className="rounded-md border bg-canvas px-3 py-2 text-ink" name="password" required type="password" />
            </label>
            <button className="rounded-md border px-4 py-2 font-semibold" type="submit">Test sign in</button>
          </form>
        ) : null}
      </section>
    </main>
  );
}
