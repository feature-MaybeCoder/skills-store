import { useCallback, useEffect, useState } from "react";

import { type SkillRef, type StoreState, skillsApi } from "@/shared/api/skills";

export function skillKey(ref: SkillRef): string {
  return `${ref.scope}:${ref.project ?? ""}:${ref.dirName}`;
}

export function useSkillsStore() {
  const [state, setState] = useState<StoreState | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async (fn: () => Promise<StoreState>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await fn();
      setState(next);
      return next;
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void run(() => skillsApi.getState());
  }, [run]);

  const toggleSelect = useCallback((ref: SkillRef) => {
    setSelected((prev) => {
      const key = skillKey(ref);
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  }, []);

  const clearSelection = useCallback(() => setSelected(new Set()), []);

  return {
    state,
    selected,
    busy,
    error,
    run,
    toggleSelect,
    clearSelection,
  };
}
