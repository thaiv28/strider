import { NextRequest, NextResponse } from "next/server";

const COOKIE_NAME = "strider_session";

export function middleware(request: NextRequest) {
  const sessionSecret = process.env.SESSION_SECRET;
  if (!sessionSecret) {
    return new NextResponse("Service authentication is not configured.", { status: 503 });
  }
  if (request.cookies.get(COOKIE_NAME)?.value === sessionSecret) return NextResponse.next();

  const loginUrl = new URL("/login", request.url);
  return NextResponse.redirect(loginUrl);
}

export const config = {
  matcher: ["/((?!login|api/login|api/health|_next/static|_next/image|icon.svg|apple-icon).*)"],
};
