import { useCallback, useEffect, useRef, useState } from 'react';

// biome-ignore lint/suspicious/noExplicitAny: SSE hook accepts various data types
export interface SSEHookOptions<T = any> {
  onMessage?: (data: T) => void;
  onError?: (error: unknown) => void;
  onOpen?: () => void;
  parseJSON?: boolean;
  params?: Record<string, string | number | boolean>;
}

// SSE (Server-Sent Events) controller hook for processing real-time events received from the backend.
// The setup streams are POST + JSON so that a cross-site form cannot trigger them, and EventSource
// can only issue GET, so the stream is read from a fetch response body instead.
// biome-ignore lint/suspicious/noExplicitAny: SSE hook accepts various data types
export function useSSEController<T = any>(
  url: string,
  params: Record<string, string | number | boolean | null>,
  options: SSEHookOptions<T> = {},
) {
  const abortRef = useRef<AbortController | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const stop = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    setIsConnected(false);
  }, []);

  const start = useCallback(() => {
    if (abortRef.current) return;

    const controller = new AbortController();
    abortRef.current = controller;

    const body = Object.fromEntries(
      Object.entries(params).filter(([, value]) => value !== undefined && value !== null),
    );

    const run = async () => {
      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal: controller.signal,
      });

      if (!response.ok || response.body === null) {
        throw new Error(`SSE request to ${url} failed with status ${response.status}`);
      }

      setIsConnected(true);
      setError(null);
      options.onOpen?.();

      const reader = response.body.pipeThrough(new TextDecoderStream()).getReader();
      // A blank line terminates an SSE frame, so the text after the last blank line is
      // an incomplete frame and has to wait for the next chunk.
      let carry = '';

      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;

        carry += value;
        const frames = carry.split('\n\n');
        carry = frames.pop() ?? '';

        for (const frame of frames) {
          const data = frame
            .split('\n')
            .filter((line) => line.startsWith('data:'))
            .map((line) => line.slice(5).replace(/^ /, ''))
            .join('\n');
          if (data.length === 0) continue;
          options.onMessage?.(
            options.parseJSON === false ? (data as T) : JSON.parse(data),
          );
        }
      }
    };

    run()
      .then(() => {
        if (abortRef.current === controller) abortRef.current = null;
        setIsConnected(false);
      })
      .catch((e: unknown) => {
        if (controller.signal.aborted) return;
        setError(e);
        setIsConnected(false);
        options.onError?.(e);
        stop();
      });
  }, [url, params, options, stop]);

  const restart = useCallback(() => {
    stop();
    start();
  }, [start, stop]);

  useEffect(() => stop, [stop]);

  return { start, stop, restart, isConnected, error };
}
