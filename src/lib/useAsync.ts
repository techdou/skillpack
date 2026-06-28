import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Async state hook that removes the `useState + useEffect + load() + error +
 * loading` boilerplate repeated across pages.
 *
 * Runs `fn` on mount (and whenever `deps` change), tracks `data`/`error`/
 * `loading`, and exposes `reload()` to re-run on demand (e.g. after a
 * mutation). `deps` must mirror every value `fn` closes over, just like
 * `useEffect` deps.
 *
 * Returned functions are stable across renders unless `deps` change, so they
 * are safe to pass as props.
 */
export function useAsync<T>(
  fn: () => Promise<T>,
  deps: ReadonlyArray<unknown> = [],
): {
  data: T | null;
  error: string | null;
  loading: boolean;
  reload: () => Promise<void>;
} {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  // Prevent setting state after unmount, and allow callers to invalidate the
  // last request by capturing a per-run token.
  const tokenRef = useRef(0);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const reload = useCallback(async () => {
    const token = ++tokenRef.current;
    setLoading(true);
    try {
      const result = await fn();
      if (token === tokenRef.current && mountedRef.current) {
        setData(result);
        setError(null);
      }
    } catch (e) {
      if (token === tokenRef.current && mountedRef.current) {
        setError(String(e));
      }
    } finally {
      if (token === tokenRef.current && mountedRef.current) {
        setLoading(false);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  useEffect(() => {
    reload();
  }, [reload]);

  return { data, error, loading, reload };
}
