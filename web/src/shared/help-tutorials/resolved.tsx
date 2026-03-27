import { useQuery } from '@tanstack/react-query';
import { useMatches } from '@tanstack/react-router';
import { useApp } from '../hooks/useApp';
import { helpTutorialsQueryOptions } from './data';
import { resolveHelpTutorials } from './resolver';
import { canonicalizeRouteKey } from './route-key';
import type { HelpTutorial } from './types';

const SHELL_ROUTE_PREFIX = '/_authorized/_default/';

/**
 * Derives the canonical route key for the current page from TanStack Router
 * matches, skipping pathless shell/layout routes.
 * Returns null if no content route is active.
 */
function useHelpTutorialsRouteKey(): string | null {
  const matches = useMatches();
  // Find the deepest match that belongs to the authorized default shell
  const contentMatch = [...matches]
    .reverse()
    .find((m) => m.routeId.startsWith(SHELL_ROUTE_PREFIX));
  if (!contentMatch) return null;
  return canonicalizeRouteKey(contentMatch.fullPath);
}

/**
 * Returns the resolved tutorial list for the current page and app version.
 * Returns an empty array when data is loading, errored, or no tutorials match.
 */
export function useResolvedHelpTutorials(): HelpTutorial[] {
  const { data } = useQuery(helpTutorialsQueryOptions);
  const appVersion = useApp((s) => s.appInfo.version);
  const routeKey = useHelpTutorialsRouteKey();

  if (!data || !appVersion || !routeKey) return [];
  return resolveHelpTutorials(data, appVersion, routeKey);
}
