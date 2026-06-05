import { useEffect, useRef, useCallback, useState } from "react";

export function useAutoSave(
  content: string,
  isDirty: boolean,
  onSave: () => Promise<void>,
  delay: number = 1000
) {
  const [isSaving, setIsSaving] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onSaveRef = useRef(onSave);
  onSaveRef.current = onSave;

  useEffect(() => {
    if (!isDirty) return;

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    timeoutRef.current = setTimeout(async () => {
      setIsSaving(true);
      try {
        await onSaveRef.current();
      } finally {
        setIsSaving(false);
      }
    }, delay);

    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, [content, isDirty, delay]);

  const triggerSave = useCallback(async () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    setIsSaving(true);
    try {
      await onSaveRef.current();
    } finally {
      setIsSaving(false);
    }
  }, []);

  return { isSaving, triggerSave };
}
