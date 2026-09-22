import type { NextAuthConfig } from "next-auth";
import Google from "next-auth/providers/google";

const PUBLIC_PATHS = new Set(["/", "/login", "/privacy", "/api/health"]);

export default {
  secret: process.env.AUTH_SECRET ?? process.env.SESSION_SECRET,
  trustHost: true,
  providers: [Google],
  pages: { signIn: "/login" },
  session: { strategy: "jwt" },
  callbacks: {
    authorized({ auth, request }) {
      const path = request.nextUrl.pathname;
      if (PUBLIC_PATHS.has(path) || path.startsWith("/share/") || path.startsWith("/api/auth/")) {
        return true;
      }
      return Boolean(auth?.user);
    },
  },
} satisfies NextAuthConfig;
