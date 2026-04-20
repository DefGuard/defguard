import { resolveVersion } from '../../utils/resolveVersion';
import type {
  ContextualHelpKey,
  ContextualHelpMappings,
  ContextualHelpPage,
} from './types';

export function resolveContextualHelpPage(
  mappings: ContextualHelpMappings,
  appVersionRaw: string,
  key: ContextualHelpKey,
): ContextualHelpPage | null {
  return resolveVersion(mappings, appVersionRaw)?.pages[key] ?? null;
}
