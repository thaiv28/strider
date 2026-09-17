"use client";

import { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { Card, Button } from "@/components/ui";

// Lets buffered forms flag unsaved edits so navigating away (in-app links or a
// browser close/refresh) prompts to save/discard first. Forms register via
// useUnsavedGuard; the Shell routes its nav clicks through requestNavigate.

type Entry = { dirty: boolean; save: () => Promise<void> | void };
type Ctx = {
  register: (key: string, e: Entry) => void;
  unregister: (key: string) => void;
  requestNavigate: (href: string) => void;
};

const UnsavedCtx = createContext<Ctx | null>(null);

export function UnsavedChangesProvider({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const reg = useRef(new Map<string, Entry>());
  const [pending, setPending] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const anyDirty = () => [...reg.current.values()].some((e) => e.dirty);
  const register = useCallback((key: string, e: Entry) => void reg.current.set(key, e), []);
  const unregister = useCallback((key: string) => void reg.current.delete(key), []);
  const requestNavigate = useCallback(
    (href: string) => (anyDirty() ? setPending(href) : router.push(href)),
    [router],
  );

  useEffect(() => {
    const h = (e: BeforeUnloadEvent) => {
      if (anyDirty()) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", h);
    return () => window.removeEventListener("beforeunload", h);
  }, []);

  const leave = () => {
    const h = pending;
    setPending(null);
    if (h) router.push(h);
  };
  const saveAndLeave = async () => {
    setSaving(true);
    try {
      await Promise.all([...reg.current.values()].filter((e) => e.dirty).map((e) => e.save()));
    } finally {
      setSaving(false);
    }
    leave();
  };

  return (
    <UnsavedCtx.Provider value={{ register, unregister, requestNavigate }}>
      {children}
      {pending && (
        <div className="fixed inset-0 z-[1200] flex items-center justify-center bg-ink/40 p-4 backdrop-blur-sm" onClick={() => !saving && setPending(null)}>
          <Card className="w-full max-w-sm p-5" onClick={(e) => e.stopPropagation()}>
            <div className="eyebrow">◇ Unsaved changes</div>
            <p className="mt-1 text-sm text-muted">You have unsaved changes. Save them before leaving?</p>
            <div className="mt-4 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setPending(null)} disabled={saving}>Stay</Button>
              <Button variant="outline" onClick={leave} disabled={saving}>Discard</Button>
              <Button onClick={saveAndLeave} disabled={saving}>{saving ? "Saving…" : "Save & leave"}</Button>
            </div>
          </Card>
        </div>
      )}
    </UnsavedCtx.Provider>
  );
}

export function useUnsaved() {
  const c = useContext(UnsavedCtx);
  if (!c) throw new Error("useUnsaved must be used within UnsavedChangesProvider");
  return c;
}

// Register a form's dirty flag + save action for the leave guard.
export function useUnsavedGuard(key: string, dirty: boolean, save: () => Promise<void> | void) {
  const { register, unregister } = useUnsaved();
  useEffect(() => {
    register(key, { dirty, save });
    return () => unregister(key);
  }, [key, dirty, save, register, unregister]);
}
