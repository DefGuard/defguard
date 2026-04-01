import { useQuery } from '@tanstack/react-query';
import { useMatches } from '@tanstack/react-router';
import { useApp } from '../hooks/useApp';
import { videoTutorialsQueryOptions } from '../query';
import { resolveAllSections, resolveVideoTutorials } from './resolver';
import { canonicalizeRouteKey } from './route-key';
import type { VideoTutorial, VideoTutorialsSection } from './types';

// Matches routes defined under src/routes/_authorized/_default.tsx
const CONTENT_ROUTE_PREFIX = '/_authorized/_default/';

// Stable empty references — avoids triggering effects/memos that depend on these values.
const EMPTY_VIDEO_TUTORIALS: VideoTutorial[] = [];
const EMPTY_SECTIONS: VideoTutorialsSection[] = [];

/**
 * Derives the canonical route key for the current page from TanStack Router
 * matches, skipping pathless shell/layout routes.
 * Returns null if no content route is active.
 */
export function useVideoTutorialsRouteKey(): string | null {
  const matches = useMatches();
  // Find the deepest match that belongs to the authorized default shell
  const contentMatch = [...matches]
    .reverse()
    .find((m) => m.routeId.startsWith(CONTENT_ROUTE_PREFIX));
  if (!contentMatch) return null;
  return canonicalizeRouteKey(contentMatch.fullPath);
}

/**
 * Returns the resolved video tutorials list for the current page and app version.
 * Returns an empty array when data is loading, errored, or no videos match.
 */
export function useResolvedVideoTutorials(): VideoTutorial[] {
  const { data } = useQuery(videoTutorialsQueryOptions);
  const appVersion = useApp((s) => s.appInfo.version);
  const routeKey = useVideoTutorialsRouteKey();

  if (!data || !appVersion || !routeKey) return EMPTY_VIDEO_TUTORIALS;
  return resolveVideoTutorials(data, appVersion, routeKey);
}

/**
 * Returns all sections from the newest eligible version for the current app version.
 * Used by VideoTutorialsModal to display the full content list.
 * Returns an empty array when data is loading, errored, or no eligible version exists.
 */
export function useAllVideoTutorialsSections(): VideoTutorialsSection[] {
  const { data } = useQuery(videoTutorialsQueryOptions);
  const appVersion = useApp((s) => s.appInfo.version);

  if (!data || !appVersion) return EMPTY_SECTIONS;
  return resolveAllSections(data, appVersion);
}
