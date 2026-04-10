import type {
  MigrationWizardPlacement,
  VideoTutorialsMappings,
  VideoTutorialsSection,
  VideoTutorialsVersionEntry,
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
 * Returns the newest eligible version entry (version key <= app version).
 * Consumers that only need sections or placements should prefer the narrower
 * helpers (`resolveSections` / `resolveMigrationWizardPlacement`). Returns
 * `null` if no eligible version exists or the app version is invalid.
 */
export function resolveVersion(
  mappings: VideoTutorialsMappings,
  appVersionRaw: string,
): VideoTutorialsVersionEntry | null {
  const appVersion = parseVersion(appVersionRaw);
  if (!appVersion) return null;

  const versions = eligibleVersionsSorted(mappings, appVersion);
  if (versions.length === 0) return null;

  return mappings[versions[0]];
}

export function resolveSections(
  mappings: VideoTutorialsMappings,
  appVersionRaw: string,
): VideoTutorialsSection[] {
  return resolveVersion(mappings, appVersionRaw)?.sections ?? [];
}

export function resolveMigrationWizardPlacement(
  mappings: VideoTutorialsMappings,
  appVersionRaw: string,
): MigrationWizardPlacement | null {
  return resolveVersion(mappings, appVersionRaw)?.placements?.migrationWizard ?? null;
}
