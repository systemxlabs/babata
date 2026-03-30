import { startTransition, useEffect, useEffectEvent, useRef, useState } from 'react';

interface UsePollingOptions {
  intervalMs?: number;
  autoRefresh?: boolean;
  immediate?: boolean;
}

interface UsePollingResult<TData> {
  data: TData | null;
  error: Error | null;
  isLoading: boolean;
  isRefreshing: boolean;
  lastRefreshedAt: number | null;
  autoRefresh: boolean;
  setAutoRefresh: (value: boolean) => void;
  refresh: () => Promise<void>;
}

export function usePolling<TData>(
  loader: () => Promise<TData>,
  options: UsePollingOptions = {},
): UsePollingResult<TData> {
  const { intervalMs = 5000, autoRefresh: autoRefreshDefault = true, immediate = true } = options;
  const [data, setData] = useState<TData | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [isLoading, setIsLoading] = useState(immediate);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [lastRefreshedAt, setLastRefreshedAt] = useState<number | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(autoRefreshDefault);
  const hasLoadedRef = useRef(false);
  const requestRef = useRef<Promise<void> | null>(null);

  const runLoad = useEffectEvent(async () => {
    if (requestRef.current) {
      return requestRef.current;
    }

    const isInitialLoad = !hasLoadedRef.current;
    if (isInitialLoad) {
      setIsLoading(true);
    } else {
      setIsRefreshing(true);
    }

    const request = loader()
      .then((nextData) => {
        hasLoadedRef.current = true;
        startTransition(() => {
          setData(nextData);
          setError(null);
          setLastRefreshedAt(Date.now());
          setIsLoading(false);
          setIsRefreshing(false);
        });
      })
      .catch((nextError: unknown) => {
        const errorValue =
          nextError instanceof Error ? nextError : new Error('Polling request failed');
        startTransition(() => {
          setError(errorValue);
          setIsLoading(false);
          setIsRefreshing(false);
        });
      })
      .finally(() => {
        requestRef.current = null;
      });

    requestRef.current = request;
    return request;
  });

  useEffect(() => {
    if (!immediate) {
      setIsLoading(false);
      return;
    }

    void runLoad();
  }, [immediate, runLoad]);

  useEffect(() => {
    if (!autoRefresh) {
      return;
    }

    const intervalId = window.setInterval(() => {
      void runLoad();
    }, intervalMs);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [autoRefresh, intervalMs, runLoad]);

  return {
    data,
    error,
    isLoading,
    isRefreshing,
    lastRefreshedAt,
    autoRefresh,
    setAutoRefresh,
    refresh: async () => {
      await runLoad();
    },
  };
}
