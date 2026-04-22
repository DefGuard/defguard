import { useQuery } from '@tanstack/react-query';
import { useApp } from '../../hooks/useApp';
import { contextualHelpQueryOptions } from '../../query';
import { resolveContextualHelpPage } from './resolver';
import type { ContextualHelpKey, ContextualHelpPage } from './types';

const EMPTY: ContextualHelpPage | null = null;

/**
 * Returns the contextual help content for the given page key resolved against
 * the current app version. Returns null when data is loading, the fetch failed,
 * no eligible version exists, or the key has no content defined.
 */
export function useContextualHelp(key: ContextualHelpKey): ContextualHelpPage | null {
  const { data } = useQuery(contextualHelpQueryOptions);
  const appVersion = useApp((s) => s.appInfo.version);

  if (!data || !appVersion) return EMPTY;
  return resolveContextualHelpPage(data, appVersion, key);
}
