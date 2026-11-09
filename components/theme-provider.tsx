"use client";

import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { THEME_KEY, type ThemePreference } from "@/lib/theme";

const ThemeContext = createContext<{
  preference: ThemePreference;
  setPreference: (preference: ThemePreference) => void;
} | null>(null);

function parsePreference(value: string | null): ThemePreference {
  return value === "dark" || value === "system" ? value : "light";
}

function applyTheme(preference: ThemePreference) {
  document.documentElement.dataset.theme = preference === "dark" ||
    (preference === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches)
    ? "dark" : "light";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preference, updatePreference] = useState<ThemePreference>("light");
  const setPreference = useCallback((next: ThemePreference) => {
    try { localStorage.setItem(THEME_KEY, next); } catch { /* Storage may be unavailable. */ }
    updatePreference(next);
    applyTheme(next);
  }, []);

  useEffect(() => {
    let initial: ThemePreference = "light";
    try { initial = parsePreference(localStorage.getItem(THEME_KEY)); } catch { /* Use light. */ }
    updatePreference(initial);
    applyTheme(initial);
    const onStorage = (event: StorageEvent) => {
      if (event.key !== THEME_KEY && event.key !== null) return;
      const next = parsePreference(event.newValue);
      updatePreference(next);
      applyTheme(next);
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  useEffect(() => {
    if (preference !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme("system");
    onChange();
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [preference]);

  return <ThemeContext.Provider value={{ preference, setPreference }}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme must be used within ThemeProvider");
  return context;
}
