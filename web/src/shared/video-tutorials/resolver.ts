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
  stepKey?: string,
): VideoGuidePlacement | null {
  if (!placementKey) {
    return null;
  }

  const placementGroup = resolveVersion(mappings, appVersionRaw)?.placements?.[
    placementKey
  ];
  if (!placementGroup) {
    return null;
  }

  if (stepKey && placementGroup.steps?.[stepKey]) {
    return placementGroup.steps[stepKey] ?? null;
  }

  return placementGroup.default ?? null;
}
