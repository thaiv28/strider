import Link from "next/link";
import { loginForE2e, loginWithGoogle } from "@/app/login/actions";
import { TripPreview } from "./trip-preview";

export function LandingPage({ signedIn, callbackUrl, error, test }: { signedIn: boolean; callbackUrl: string; error: boolean; test: boolean }) {
  return (
    <div className="landing relative isolate min-h-dvh bg-header text-headerink">
      <svg className="pointer-events-none fixed inset-0 -z-10 h-full w-full" aria-hidden="true" viewBox="0 0 1440 900" preserveAspectRatio="xMidYMid slice"><g className="landing-contours" fill="none" stroke="var(--header-line)" strokeWidth="1.2"><path d="M-40 520C200 470 330 600 520 540S860 460 1480 560"/><path d="M-40 590C210 540 340 670 540 610S880 530 1480 630"/><path d="M-40 660C220 610 350 740 560 680S900 600 1480 700"/><path d="M-40 730C230 680 360 810 580 750S920 670 1480 770"/><path d="M-40 120C240 70 420 170 720 110S1180 60 1480 130"/><path d="M-40 180C260 130 440 230 740 170S1200 120 1480 190"/></g></svg>
      <main className="mx-auto grid max-w-[1600px] gap-7 px-4 pb-2 pt-5 sm:px-[clamp(20px,5vw,72px)] sm:py-[clamp(24px,3vw,40px)] desk:min-h-dvh desk:grid-cols-[minmax(300px,380px)_minmax(0,1fr)] desk:gap-[clamp(32px,4vw,64px)]">
        <div className="contents desk:flex desk:flex-col desk:justify-between desk:gap-8">
          <Link href="/" className="font-display text-2xl font-bold">Strider</Link>
          <div>
            <h1 className="font-display text-[40px] font-extrabold leading-none tracking-[-0.035em] sm:text-[52px] desk:text-[64px]">Plan your backpacking trip in one place.</h1>
            {signedIn ? <Link href="/basecamp" className="mt-8 inline-flex min-h-12 items-center rounded-xl bg-headerink px-5 font-semibold text-headeractionink hover:opacity-90">Open Strider</Link> : (
              <form action={loginWithGoogle} className="mt-8">
                <input type="hidden" name="callbackUrl" value={callbackUrl} />
                <button type="submit" className="inline-flex min-h-12 w-full sm:w-auto items-center justify-center gap-3 rounded-md border border-[#dadce0] bg-white px-4 py-2.5 font-[system-ui,sans-serif] text-sm font-medium text-[#1f1f1f] shadow-sm transition hover:bg-[#f8f9fa] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent">
                  <GoogleMark />Sign in with Google
                </button>
              </form>
            )}
            {error && <p role="alert" className="mt-4 text-sm text-headeralert">Google sign-in didn't finish. Try again.</p>}
            {test && <form action={loginForE2e} className="mt-6 grid gap-3 border-t border-headerborder pt-5 text-left">
              <input type="hidden" name="callbackUrl" value={callbackUrl} />
              <label className="grid gap-1.5 text-sm font-semibold">Test account
                <input className="min-h-11 rounded-[10px] border border-headerborder bg-headeractive px-3 py-2 text-headerink" defaultValue="e2e@strider.invalid" name="email" required type="email" />
              </label>
              <label className="grid gap-1.5 text-sm font-semibold">Test password
                <input className="min-h-11 rounded-[10px] border border-headerborder bg-headeractive px-3 py-2 text-headerink" name="password" required type="password" />
              </label>
              <button className="min-h-11 rounded-xl border border-headerborder px-4 py-2 font-semibold hover:bg-headeractive" type="submit">Test sign in</button>
            </form>}
          </div>
          <Link href="/privacy" className="order-3 inline-flex min-h-11 w-max items-center text-[13px] text-headermuted underline-offset-4 hover:text-headerink hover:underline desk:order-none">Privacy</Link>
        </div>
        <TripPreview />
      </main>
    </div>
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
