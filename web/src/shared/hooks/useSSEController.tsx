import { useCallback, useEffect, useRef, useState } from 'react';

// biome-ignore lint/suspicious/noExplicitAny: SSE hook accepts various data types
export interface SSEHookOptions<T = any> {
  onMessage?: (data: T) => void;
  onError?: (error: Event) => void;
  onOpen?: () => void;
  parseJSON?: boolean;
  params?: Record<string, string | number | boolean>;
}

// SSE (Server-Sent Events) controller hook for processing real-time events received from the backend.
// biome-ignore lint/suspicious/noExplicitAny: SSE hook accepts various data types
export function useSSEController<T = any>(
  url: string,
  params: Record<string, string | number | boolean | null>,
  options: SSEHookOptions<T> = {},
) {
  const eventSourceRef = useRef<EventSource | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<Event | null>(null);

  const buildUrl = useCallback(() => {
    const qs = new URLSearchParams();
    Object.entries(params).forEach(([k, v]) => {
      if (v !== undefined && v !== null) qs.append(k, String(v));
    });
    return qs.toString() ? `${url}?${qs}` : url;
  }, [url, params]);

  const stop = useCallback(() => {
    eventSourceRef.current?.close();
    eventSourceRef.current = null;
    setIsConnected(false);
  }, []);

  const start = useCallback(() => {
    if (eventSourceRef.current) return;

    const es = new EventSource(buildUrl());
    eventSourceRef.current = es;

    es.onopen = () => {
      setIsConnected(true);
      setError(null);
      options.onOpen?.();
    };

    es.onmessage = (e) => {
      const data = options.parseJSON === false ? e.data : JSON.parse(e.data);
      options.onMessage?.(data);
    };

    es.onerror = (e) => {
      setError(e);
      setIsConnected(false);
      options.onError?.(e);
      stop();
    };
  }, [buildUrl, options, stop]);

  const restart = useCallback(() => {
    stop();
    start();
  }, [start, stop]);

  useEffect(() => stop, [stop]);

  return { start, stop, restart, isConnected, error };
}
