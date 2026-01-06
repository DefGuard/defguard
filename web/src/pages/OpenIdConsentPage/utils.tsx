import type { OpenIdClientScopeValue } from '../../shared/api/types';

export const parseOpenIdScopeSearch = (scope: string): OpenIdClientScopeValue[] => {
  if (!scope) return [];

  return Array.from(
    new Set(
      scope
        .split(/[ ,]+/)
        .map((s) => s.trim())
        .filter(Boolean),
    ),
  ) as Array<OpenIdClientScopeValue>;
};
