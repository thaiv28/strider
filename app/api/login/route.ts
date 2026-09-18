import { createHash, timingSafeEqual } from "node:crypto";
import { NextRequest, NextResponse } from "next/server";

const COOKIE_NAME = "strider_session";

function passwordMatches(candidate: string): boolean {
  const expected = process.env.APP_PASSWORD_HASH;
  if (!expected || !/^[a-f0-9]{64}$/i.test(expected)) return false;

  const actualBuffer = createHash("sha256").update(candidate).digest();
  const expectedBuffer = Buffer.from(expected, "hex");
  return expectedBuffer.length === actualBuffer.length && timingSafeEqual(actualBuffer, expectedBuffer);
}

export async function POST(request: NextRequest) {
  const formData = await request.formData();
  const password = String(formData.get("password") ?? "");
  const sessionSecret = process.env.SESSION_SECRET;

  if (!sessionSecret || !passwordMatches(password)) {
    return NextResponse.redirect(new URL("/login?error=1", request.url), 303);
  }

  const response = NextResponse.redirect(new URL("/", request.url), 303);
  response.cookies.set(COOKIE_NAME, sessionSecret, {
    httpOnly: true,
    maxAge: 60 * 60 * 24 * 30,
    path: "/",
    sameSite: "strict",
    secure: process.env.NODE_ENV === "production",
  });
  return response;
}
