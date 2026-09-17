"use server";

import { createHash, timingSafeEqual } from "node:crypto";
import { cookies } from "next/headers";
import { redirect } from "next/navigation";

const COOKIE_NAME = "strider_session";

function passwordMatches(candidate: string): boolean {
  const expected = process.env.APP_PASSWORD_HASH;
  if (!expected || !/^[a-f0-9]{64}$/i.test(expected)) return false;
  const actualBuffer = createHash("sha256").update(candidate).digest();
  const expectedBuffer = Buffer.from(expected, "hex");
  return expectedBuffer.length === actualBuffer.length && timingSafeEqual(actualBuffer, expectedBuffer);
}

export async function login(formData: FormData) {
  const password = String(formData.get("password") ?? "");
  const sessionSecret = process.env.SESSION_SECRET;
  if (!sessionSecret || !passwordMatches(password)) {
    redirect("/login?error=1");
  }

  const jar = await cookies();
  jar.set(COOKIE_NAME, sessionSecret, {
    httpOnly: true,
    maxAge: 60 * 60 * 24 * 30,
    path: "/",
    sameSite: "strict",
    secure: process.env.NODE_ENV === "production",
  });
  redirect("/");
}

export async function logout() {
  const jar = await cookies();
  jar.delete(COOKIE_NAME);
  redirect("/login");
}
