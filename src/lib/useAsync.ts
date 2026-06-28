import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Async state hook that removes the `useState + useEffect + load() + error +
 * loading` boilerplate repeated across pages.
 *
 * Runs `fn` on mount (and whenever `deps` change), tracks `data`/`error`/
 * `loading`, and exposes `reload()` to re-run on demand (e.g. after a
 * mutation).
 *
 * `fn` is captured by ref, so callers may pass an inline closure without
 * worrying that `reload` will call a stale copy — the latest `fn` always runs.
 * `deps` controls *when* the hook re-runs on mount/update (mirroring
 * `useEffect` deps), not which `fn` is invoked.
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

  // Keep the latest fn in a ref so reload always invokes the current closure,
  // even if the caller passes an inline arrowhead that changes each render.
  // Without this, a stale fn would be captured by the memoised reload.
  const fnRef = useRef(fn);
  fnRef.current = fn;

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
      const result = await fnRef.current();
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
  }, []);

  useEffect(() => {
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return { data, error, loading, reload };
}
