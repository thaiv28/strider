import { createHash, timingSafeEqual } from "node:crypto";
import { and, eq, isNull, notLike } from "drizzle-orm";
import NextAuth from "next-auth";
import Credentials from "next-auth/providers/credentials";
import authConfig from "@/auth.config";
import { db, schema } from "@/src/db/index";

const E2E_EMAIL = "e2e@strider.invalid";

function matchesE2ePassword(candidate: string): boolean {
  const expected = process.env.AUTH_E2E_PASSWORD_HASH ?? process.env.APP_PASSWORD_HASH;
  if (!expected || !/^[a-f0-9]{64}$/i.test(expected)) return false;
  const actualBuffer = createHash("sha256").update(candidate).digest();
  const expectedBuffer = Buffer.from(expected, "hex");
  return expectedBuffer.length === actualBuffer.length && timingSafeEqual(actualBuffer, expectedBuffer);
}

async function ensureE2eUser(email: string) {
  const [existing] = await db.select().from(schema.users).where(eq(schema.users.email, email));
  if (existing) return existing;
  const [created] = await db
    .insert(schema.users)
    .values({ email, name: "Strider E2E" })
    .onConflictDoNothing({ target: schema.users.email })
    .returning();
  if (created) return created;
  return (await db.select().from(schema.users).where(eq(schema.users.email, email)))[0];
}

async function linkGoogleUser(profile: Record<string, unknown>) {
  const subject = typeof profile.sub === "string" ? profile.sub : null;
  const email = typeof profile.email === "string" ? profile.email.trim().toLowerCase() : null;
  const verified = profile.email_verified === true;
  if (!subject || !email || !verified) return null;

  const [bySubject] = await db.select().from(schema.users).where(eq(schema.users.googleSubject, subject));
  if (bySubject) return bySubject;

  const name = typeof profile.name === "string" ? profile.name : null;
  const imageUrl = typeof profile.picture === "string" ? profile.picture : null;
  const [byEmail] = await db.select().from(schema.users).where(eq(schema.users.email, email));
  if (byEmail?.googleSubject && byEmail.googleSubject !== subject) return null;

  // The configured owner may claim the one pre-OAuth data user without changing
  // its id, so every existing foreign key remains attached to the same account.
  const ownerEmail = process.env.AUTH_OWNER_EMAIL?.trim().toLowerCase();
  if (ownerEmail === email) {
    const legacy = await db
      .select()
      .from(schema.users)
      .where(and(isNull(schema.users.googleSubject), notLike(schema.users.email, "%@strider.invalid")))
      .limit(2);
    if (legacy.length === 1) {
      // Refuse instead of guessing if this email already names a different row.
      if (byEmail && byEmail.id !== legacy[0].id) return null;
      const [linked] = await db
        .update(schema.users)
        .set({ email, googleSubject: subject, name: name ?? legacy[0].name, imageUrl })
        .where(eq(schema.users.id, legacy[0].id))
        .returning();
      return linked;
    }
    if (legacy.length > 1) return null;
  }

  if (byEmail) {
    const [linked] = await db
      .update(schema.users)
      .set({ googleSubject: subject, name: name ?? byEmail.name, imageUrl })
      .where(eq(schema.users.id, byEmail.id))
      .returning();
    return linked;
  }

  const [created] = await db
    .insert(schema.users)
    .values({ email, name, googleSubject: subject, imageUrl })
    .returning();
  return created;
}

export const { handlers, auth, signIn, signOut } = NextAuth({
  ...authConfig,
  providers: [
    ...authConfig.providers,
    Credentials({
      id: "e2e",
      name: "Strider test account",
      credentials: {
        email: { type: "email" },
        password: { type: "password" },
      },
      async authorize(credentials) {
        const password = typeof credentials.password === "string" ? credentials.password : "";
        if (!matchesE2ePassword(password)) return null;
        const requestedEmail = typeof credentials.email === "string" ? credentials.email.trim().toLowerCase() : E2E_EMAIL;
        const email = /^[a-z0-9._-]+@strider\.invalid$/.test(requestedEmail) ? requestedEmail : E2E_EMAIL;
        const user = await ensureE2eUser(email);
        return user ? { id: String(user.id), email: user.email, name: user.name } : null;
      },
    }),
  ],
  callbacks: {
    ...authConfig.callbacks,
    async signIn({ account, profile }) {
      if (account?.provider === "e2e") return true;
      if (account?.provider !== "google" || !profile) return false;
      return Boolean(await linkGoogleUser(profile));
    },
    async jwt({ token, account, profile, user }) {
      if (account?.provider === "e2e" && user?.id) token.userId = Number(user.id);
      if (account?.provider === "google" && profile?.sub) {
        const [dbUser] = await db
          .select({ id: schema.users.id })
          .from(schema.users)
          .where(eq(schema.users.googleSubject, profile.sub));
        token.userId = dbUser?.id;
      }
      return token;
    },
    async session({ session, token }) {
      if (session.user && typeof token.userId === "number") session.user.id = String(token.userId);
      return session;
    },
  },
});
