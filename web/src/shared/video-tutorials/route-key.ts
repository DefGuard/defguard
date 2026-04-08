/**
 * Canonicalize a route key:
 * - Trim surrounding whitespace
 * - Ensure a leading "/"
 * - Remove trailing "/" unless the result would be empty (root stays "/")
 */
export function canonicalizeRouteKey(raw: string): string {
  let key = raw.trim();
  if (!key.startsWith('/')) {
    key = `/${key}`;
  }
  if (key.length > 1 && key.endsWith('/')) {
    key = key.slice(0, -1);
  }
  return key;
}

/**
 * Strips dynamic segments from a canonical route path, returning the longest
 * static prefix that can be used for navigation and label lookup.
 *
 * Dynamic segments use TanStack Router's "$param" convention. Everything from
 * the first "/$"-prefixed segment onward is removed.
 *
 * @example
 *   getNavRoot('/vpn-overview')              // → '/vpn-overview'
 *   getNavRoot('/vpn-overview/$locationId')  // → '/vpn-overview'
 *   getNavRoot('/acl/rules/$ruleId/edit')    // → '/acl/rules'
 *   getNavRoot('/$id')                       // → '/'
 */
export function getNavRoot(route: string): string {
  const canonical = canonicalizeRouteKey(route);
  const paramIdx = canonical.indexOf('/$');
  return paramIdx === -1 ? canonical : canonical.slice(0, paramIdx) || '/';
}
