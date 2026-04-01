import { canonicalizeRouteKey } from './route-key';
import type {
  VideoTutorial,
  VideoTutorialsMappings,
  VideoTutorialsSection,
} from './types';
import { compareVersions, parseVersion } from './version';

/**
 * Returns the sorted list of version keys that are eligible for the given app
 * version (i.e. version key <= app version), ordered newest-to-oldest.
 */
function eligibleVersionsSorted(
  mappings: VideoTutorialsMappings,
  appVersion: NonNullable<ReturnType<typeof parseVersion>>,
): string[] {
  return Object.keys(mappings)
    .flatMap((key) => {
      const parsed = parseVersion(key);
      return parsed && compareVersions(parsed, appVersion) <= 0 ? [{ key, parsed }] : [];
    })
    .sort((a, b) => compareVersions(b.parsed, a.parsed))
    .map(({ key }) => key);
}

/**
 * Given the parsed video tutorials mappings, the current app version string, and the
 * current normalized route key, returns the videos from the newest eligible version
 * that has at least one video matching the route.
 *
 * Resolution rules:
 * - Only version keys that are <= the runtime app version are eligible.
 * - Eligible versions are walked newest-to-oldest.
 * - The first version that has any video whose canonicalized appRoute matches the
 *   route key wins; those matching videos are returned.
 * - If no version has a matching video, returns [].
 * - If the app version or route key is invalid/missing, returns [].
 */
export function resolveVideoTutorials(
  mappings: VideoTutorialsMappings,
  appVersionRaw: string,
  routeKey: string,
): VideoTutorial[] {
  const appVersion = parseVersion(appVersionRaw);
  if (!appVersion) return [];

  for (const versionKey of eligibleVersionsSorted(mappings, appVersion)) {
    const matched: VideoTutorial[] = [];
    for (const section of mappings[versionKey]) {
      for (const video of section.videos) {
        if (canonicalizeRouteKey(video.appRoute) === routeKey) {
          matched.push(video);
        }
      }
    }
    if (matched.length > 0) return matched;
  }

  return [];
}

/**
 * Returns all sections from the newest eligible version (version key <= app version).
 * Used by the VideoTutorialsModal to display all available content.
 * Returns [] if no eligible version exists or the app version is invalid.
 */
export function resolveAllSections(
  mappings: VideoTutorialsMappings,
  appVersionRaw: string,
): VideoTutorialsSection[] {
  const appVersion = parseVersion(appVersionRaw);
  if (!appVersion) return [];

  const versions = eligibleVersionsSorted(mappings, appVersion);
  if (versions.length === 0) return [];

  return mappings[versions[0]];
}
