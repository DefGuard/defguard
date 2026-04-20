import { resolveVersion } from '../utils/resolveVersion';
import type {
  VideoGuidePlacement,
  VideoTutorialsMappings,
  VideoTutorialsSection,
} from './types';

export function resolveSections(
  mappings: VideoTutorialsMappings,
  appVersionRaw: string,
): VideoTutorialsSection[] {
  return resolveVersion(mappings, appVersionRaw)?.sections ?? [];
}

export function resolveVideoGuidePlacement(
  mappings: VideoTutorialsMappings,
  appVersionRaw: string,
  placementKey: string | undefined,
): VideoGuidePlacement | null {
  if (!placementKey) {
    return null;
  }

  return resolveVersion(mappings, appVersionRaw)?.placements?.[placementKey] ?? null;
}
