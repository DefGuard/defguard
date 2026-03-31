import { useQuery } from '@tanstack/react-query';
import { useMatches } from '@tanstack/react-router';
import { useApp } from '../hooks/useApp';
import { videoSupportQueryOptions } from '../query';
import { resolveVideoSupport } from './resolver';
import { canonicalizeRouteKey } from './route-key';
import type { VideoSupport } from './types';

// Matches routes defined under src/routes/_authorized/_default.tsx
const CONTENT_ROUTE_PREFIX = '/_authorized/_default/';

// Stable empty reference — avoids triggering effects/memos that depend on this value.
const EMPTY_VIDEO_SUPPORT: VideoSupport[] = [];

/**
 * Derives the canonical route key for the current page from TanStack Router
 * matches, skipping pathless shell/layout routes.
 * Returns null if no content route is active.
 */
export function useVideoSupportRouteKey(): string | null {
  const matches = useMatches();
  // Find the deepest match that belongs to the authorized default shell
  const contentMatch = [...matches]
    .reverse()
    .find((m) => m.routeId.startsWith(CONTENT_ROUTE_PREFIX));
  if (!contentMatch) return null;
  return canonicalizeRouteKey(contentMatch.fullPath);
}

/**
 * Returns the resolved video support list for the current page and app version.
 * Returns an empty array when data is loading, errored, or no videos match.
 */
export function useResolvedVideoSupport(): VideoSupport[] {
  const { data } = useQuery(videoSupportQueryOptions);
  const appVersion = useApp((s) => s.appInfo.version);
  const routeKey = useVideoSupportRouteKey();

  if (!data || !appVersion || !routeKey) return EMPTY_VIDEO_SUPPORT;
  return resolveVideoSupport(data, appVersion, routeKey);
}
