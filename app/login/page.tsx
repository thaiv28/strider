import Link from "next/link";
import { auth } from "@/auth";
import { loginForE2e, loginWithGoogle } from "./actions";
import { redirect } from "next/navigation";

export const metadata = { title: "Sign in" };

export default async function LoginPage({
  searchParams,
}: {
  searchParams: Promise<{ error?: string; test?: string; callbackUrl?: string }>;
}) {
  const { error, test, callbackUrl } = await searchParams;
  const returnTo = callbackUrl?.startsWith("/") && !callbackUrl.startsWith("//") && !callbackUrl.includes("\\") ? callbackUrl : "/basecamp";
  if (await auth()) redirect(returnTo);
  return (
    <main className="flex min-h-screen flex-col items-center justify-center px-4 py-12">
      <Link href="/" className="font-display text-lg font-bold tracking-[0.16em]">STRIDER</Link>
      <section className="card mt-6 w-full max-w-sm p-6 text-center sm:p-8">
        <h1 className="text-xl font-semibold">Sign in to Strider</h1>
        <p className="mt-2 text-sm text-muted">Use your Google account to continue.</p>
        <form action={loginWithGoogle} className="mt-6">
          <input type="hidden" name="callbackUrl" value={returnTo} />
          <button
            className="flex w-full items-center justify-center gap-3 rounded-md border border-[#dadce0] bg-white px-4 py-2.5 font-[system-ui,sans-serif] text-sm font-medium text-[#1f1f1f] shadow-sm transition hover:bg-[#f8f9fa] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
            type="submit"
          >
            <GoogleMark />
            Sign in with Google
          </button>
        </form>
        {error ? <p className="mt-4 text-sm text-red-700 dark:text-red-300">Google sign-in could not be completed. Please try again.</p> : null}
        {test === "1" ? (
          <form action={loginForE2e} className="mt-6 grid gap-3 border-t pt-5 text-left">
            <input type="hidden" name="callbackUrl" value={returnTo} />
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
      <p className="mt-6 text-xs text-muted">
        New here? Signing in creates your private workspace.{" "}
        <Link href="/privacy" className="underline hover:text-ink">Privacy</Link>
      </p>
    </main>
  );
}

function GoogleMark() {
  return (
    <svg aria-hidden="true" width="18" height="18" viewBox="0 0 48 48">
      <path fill="#EA4335" d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z" />
      <path fill="#4285F4" d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6c4.51-4.18 7.09-10.36 7.09-17.65z" />
      <path fill="#FBBC05" d="M10.53 28.59c-.48-1.45-.76-2.99-.76-4.59s.27-3.14.76-4.59l-7.98-6.19C.92 16.46 0 20.12 0 24c0 3.88.92 7.54 2.56 10.78l7.97-6.19z" />
      <path fill="#34A853" d="M24 48c6.48 0 11.93-2.13 15.89-5.81l-7.73-6c-2.15 1.45-4.92 2.3-8.16 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z" />
    </svg>
  );
}
