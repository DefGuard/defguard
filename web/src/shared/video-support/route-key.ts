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
