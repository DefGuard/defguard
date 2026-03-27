import type { HelpTutorial, HelpTutorialsMappings } from './types';
import { compareVersions, parseVersion } from './version';

/**
 * Given the parsed tutorial mappings, the current app version string, and the
 * current normalized route key, returns the best matching tutorial list.
 *
 * Resolution rules:
 * - Only version keys that are <= the runtime app version are eligible.
 * - Eligible versions are walked newest-to-oldest.
 * - The first version that defines the route key wins (even if its value is an empty array).
 * - If no version defines the route key, returns [].
 * - If the app version or route key is invalid/missing, returns [].
 */
export function resolveHelpTutorials(
  mappings: HelpTutorialsMappings,
  appVersionRaw: string,
  routeKey: string,
): HelpTutorial[] {
  const appVersion = parseVersion(appVersionRaw);
  if (!appVersion) return [];

  // Collect and sort eligible version keys (newest first)
  const eligibleVersions = Object.keys(mappings)
    .map((key) => ({ key, parsed: parseVersion(key) }))
    .filter(
      (
        entry,
      ): entry is { key: string; parsed: NonNullable<ReturnType<typeof parseVersion>> } =>
        entry.parsed !== null && compareVersions(entry.parsed, appVersion) <= 0,
    )
    .sort((a, b) => compareVersions(b.parsed, a.parsed));

  for (const { key } of eligibleVersions) {
    const routeMap = mappings[key];
    if (routeKey in routeMap) {
      return routeMap[routeKey];
    }
  }

  return [];
}
