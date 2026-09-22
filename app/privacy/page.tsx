import Link from "next/link";

export const metadata = {
  title: "Privacy Policy",
  description: "How Strider collects, uses, shares, and protects account and trip-planning data.",
};

const sections = [
  {
    title: "Information Strider collects",
    body: (
      <>
        <p>When you sign in with Google, Strider receives your Google account identifier, verified email address, name, and profile image. Strider does not receive or store your Google password.</p>
        <p>Strider also stores the information you choose to add, including trips, routes, gear, meals, planning details, packing lists, calendar-feed URLs, and uploaded GPX or permit files.</p>
      </>
    ),
  },
  {
    title: "How information is used",
    body: <p>Information is used to authenticate you, maintain your private Strider workspace, calculate and display trip-planning information, provide requested integrations, and operate, secure, test, and improve the service.</p>,
  },
  {
    title: "Sharing and disclosure",
    body: (
      <>
        <p>Strider does not sell personal information.</p>
        <p>If you create a view-only Trip link, anyone with that bearer link can view the live shared Trip plan until you revoke it. Shared views exclude the private Trip Report field, but may include other trip logistics, gear, food, route, permit, and planning information that you intentionally place in the shared plan.</p>
        <p>Limited data may be processed by infrastructure and feature providers, including Google for authentication, AWS for hosting and encrypted backups, and mapping, weather, or nutrition services when you use those features.</p>
      </>
    ),
  },
  {
    title: "Data retention and deletion",
    body: <p>Workspace data is retained while your account is active and as needed to operate the service. You may request access, correction, export, or deletion using the support email displayed on Strider&apos;s Google authorization screen. Deleted information may remain temporarily in encrypted backups until those backups expire.</p>,
  },
  {
    title: "Security",
    body: <p>Strider uses encrypted transport, access controls, per-user authorization checks, and encrypted database backups. No internet service can guarantee absolute security, so keep view-only links and private calendar-feed URLs confidential.</p>,
  },
  {
    title: "Google API data",
    body: <p>Strider currently requests only the Google identity scopes needed for sign-in: OpenID, email, and profile. It does not currently request Google Calendar access. Any future Calendar connection will use a separate consent flow and this policy will be updated before that access is introduced.</p>,
  },
];

export default function PrivacyPage() {
  return (
    <div className="min-h-screen bg-[#f4f0e7] text-[#292520]">
      <header className="border-b border-black/10 bg-[#101713] text-[#f4f1e8]">
        <div className="mx-auto flex max-w-4xl items-center justify-between px-5 py-5 sm:px-8">
          <Link href="/" className="font-display text-lg font-bold tracking-[0.16em]">STRIDER</Link>
          <Link href="/login" className="rounded-full border border-white/20 bg-white/10 px-4 py-2 text-sm font-medium transition hover:bg-white/15">Enter Strider</Link>
        </div>
      </header>

      <main className="mx-auto max-w-4xl px-5 py-14 sm:px-8 sm:py-20">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[#397461]">Legal</p>
        <h1 className="font-display mt-3 text-4xl font-bold tracking-tight sm:text-6xl">Privacy Policy</h1>
        <p className="mt-4 text-sm text-[#717268]">Last updated September 22, 2026</p>
        <p className="mt-8 max-w-3xl text-lg leading-relaxed text-[#565850]">Strider is a backpacking-planning service. This policy explains what information Strider collects, why it is used, and the choices available to you.</p>

        <div className="mt-12 divide-y divide-black/10 border-y border-black/10">
          {sections.map((section) => (
            <section key={section.title} className="grid gap-4 py-8 sm:grid-cols-[0.8fr_1.7fr] sm:gap-10">
              <h2 className="font-display text-2xl font-bold">{section.title}</h2>
              <div className="space-y-4 leading-relaxed text-[#565850]">{section.body}</div>
            </section>
          ))}
        </div>

        <section className="mt-12 rounded-2xl bg-[#142019] p-7 text-[#f4f1e8] sm:p-9">
          <h2 className="font-display text-2xl font-bold">Questions or requests</h2>
          <p className="mt-3 leading-relaxed text-white/70">Contact Strider using the user-support email displayed on the Google sign-in and authorization screen. Include the Google email associated with your account so the request can be verified.</p>
        </section>
      </main>
    </div>
  );
}
