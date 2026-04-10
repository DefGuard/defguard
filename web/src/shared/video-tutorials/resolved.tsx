import { useQuery } from '@tanstack/react-query';
import { useMatches } from '@tanstack/react-router';
import { useApp } from '../hooks/useApp';
import { videoTutorialsQueryOptions } from '../query';
import { resolveSections, resolveVideoGuidePlacement } from './resolver';
import { canonicalizeRouteKey } from './route-key';
import type { VideoGuidePlacement, VideoTutorial, VideoTutorialsSection } from './types';

// Matches routes defined under src/routes/_authorized/_default.tsx
const CONTENT_ROUTE_PREFIX = '/_authorized/_default/';

// Stable empty references — avoids triggering effects/memos that depend on these values.
const EMPTY_VIDEO_TUTORIALS: VideoTutorial[] = [];
const EMPTY_SECTIONS: VideoTutorialsSection[] = [];
const EMPTY_VIDEO_GUIDE_PLACEMENT: VideoGuidePlacement | null = null;

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
 * Returns all sections from the newest eligible version for the current app version.
 * Used by both VideoTutorialsModal and VideoSupportWidget — both display content
 * from the same single version with no fallback to older versions.
 * Returns an empty array when data is loading, errored, or no eligible version exists.
 */
export function useVideoTutorialsSections(): VideoTutorialsSection[] {
  const { data } = useQuery(videoTutorialsQueryOptions);
  const appVersion = useApp((s) => s.appInfo.version);

  if (!data || !appVersion) return EMPTY_SECTIONS;
  return resolveSections(data, appVersion);
}

export function useWizardVideoGuidePlacement(
  placementKey: string | undefined,
): VideoGuidePlacement | null {
  const { data } = useQuery(videoTutorialsQueryOptions);
  const appVersion = useApp((s) => s.appInfo.version);

  if (!placementKey || !data || !appVersion) return EMPTY_VIDEO_GUIDE_PLACEMENT;
  return resolveVideoGuidePlacement(data, appVersion, placementKey);
}

/**
 * Returns the video tutorials for the current page, filtered from the newest
 * eligible version. Returns an empty array when data is loading, errored, no
 * eligible version exists, or no videos match the current route.
 */
export function useResolvedVideoTutorials(): VideoTutorial[] {
  const sections = useVideoTutorialsSections();
  const routeKey = useVideoTutorialsRouteKey();

  if (!routeKey) return EMPTY_VIDEO_TUTORIALS;
  return sections
    .flatMap((s) => s.videos)
    .filter((v) => canonicalizeRouteKey(v.appRoute) === routeKey);
}
