"use server";

import { signIn, signOut } from "@/auth";

export async function loginWithGoogle() {
  await signIn("google", { redirectTo: "/basecamp" });
}

export async function loginForE2e(formData: FormData) {
  await signIn("e2e", {
    email: String(formData.get("email") ?? "e2e@strider.invalid"),
    password: String(formData.get("password") ?? ""),
    redirectTo: "/basecamp",
  });
}

export async function logout() {
  await signOut({ redirectTo: "/login" });
}
