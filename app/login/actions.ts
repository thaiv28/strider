"use server";

import { signIn, signOut } from "@/auth";

function returnPath(value: FormDataEntryValue | null) {
  const path = String(value ?? "");
  return path.startsWith("/") && !path.startsWith("//") && !path.includes("\\") ? path : "/basecamp";
}

export async function loginWithGoogle(formData: FormData) {
  await signIn("google", { redirectTo: returnPath(formData.get("callbackUrl")) });
}

export async function loginForE2e(formData: FormData) {
  await signIn("e2e", {
    email: String(formData.get("email") ?? "e2e@strider.invalid"),
    password: String(formData.get("password") ?? ""),
    redirectTo: returnPath(formData.get("callbackUrl")),
  });
}

export async function logout() {
  await signOut({ redirectTo: "/login" });
}
