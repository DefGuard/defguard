export interface ParsedVersion {
  major: number;
  minor: number;
  patch: number;
}

/**
 * Parse a version string into major/minor/patch components.
 * Strips prerelease and build metadata before parsing so that
 * runtime versions like "2.2.0-beta+build.1" resolve correctly.
 * Returns null for strings that don't match after stripping.
 */
export function parseVersion(raw: string): ParsedVersion | null {
  // Strip prerelease (-...) and build (+...) suffixes
  const clean = raw.trim().replace(/[-+].*$/, '');
  const match = clean.match(/^(\d+)\.(\d+)(?:\.(\d+))?$/);
  if (!match) return null;
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: match[3] !== undefined ? parseInt(match[3], 10) : 0,
  };
}

/**
 * Compare two parsed versions.
 * Returns positive if a > b, negative if a < b, 0 if equal.
 * Suitable for sorting newest-first with `.sort((a, b) => compareVersions(b, a))`.
 */
export function compareVersions(a: ParsedVersion, b: ParsedVersion): number {
  if (a.major !== b.major) return a.major - b.major;
  if (a.minor !== b.minor) return a.minor - b.minor;
  return a.patch - b.patch;
}
