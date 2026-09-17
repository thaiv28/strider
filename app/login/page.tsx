import { login } from "./actions";

export const metadata = { title: "Sign in" };

export default async function LoginPage({
  searchParams,
}: {
  searchParams: Promise<{ error?: string }>;
}) {
  const { error } = await searchParams;
  return (
    <main className="grid min-h-screen place-items-center px-4 py-12">
      <section className="card w-full max-w-sm p-6 sm:p-8">
        <p className="eyebrow">Private trail log</p>
        <h1 className="mt-2 font-display text-3xl font-bold">STRIDER</h1>
        <p className="mt-3 text-sm text-muted">Sign in to open the backpacking planner.</p>
        <form action={login} className="mt-6 grid gap-4">
          <label className="grid gap-1.5 text-sm font-semibold">
            Password
            <input
              autoComplete="current-password"
              autoFocus
              className="rounded-md border bg-canvas px-3 py-2 text-ink outline-none focus:border-accent"
              name="password"
              required
              type="password"
            />
          </label>
          {error ? <p className="text-sm text-red-700 dark:text-red-300">Incorrect password.</p> : null}
          <button className="rounded-md bg-accent px-4 py-2 font-semibold text-accentink" type="submit">
            Sign in
          </button>
        </form>
      </section>
    </main>
  );
}
