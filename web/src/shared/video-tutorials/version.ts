// Re-export version utilities from the shared utils module so that existing
// consumers of this path continue to work without changes.

export type { ParsedVersion } from '../utils/resolveVersion';
export { compareVersions, parseVersion } from '../utils/resolveVersion';
