import { m } from '../../paraglide/messages';
import { canonicalizeRouteKey } from './route-key';

/**
 * Maps a canonicalized app route to a human-readable navigation label.
 *
 * Uses the same m.cmp_nav_item_*() calls as the Navigation component but
 * defined inline here to avoid a circular import with Navigation.tsx.
 *
 * The map is built lazily on first call (labels are functions, so they are
 * safe to call at module init time, but we defer to keep import side-effects
 * minimal).
 */
let _labelMap: Map<string, () => string> | undefined;

function getLabelMap(): Map<string, () => string> {
  if (_labelMap) return _labelMap;

  _labelMap = new Map([
    ['/vpn-overview', () => m.cmp_nav_item_overview()],
    ['/locations', () => m.cmp_nav_item_locations()],
    ['/users', () => m.cmp_nav_item_users()],
    ['/groups', () => m.cmp_nav_item_groups()],
    ['/enrollment', () => m.cmp_nav_item_enrollment()],
    ['/acl/rules', () => m.cmp_nav_item_rules()],
    ['/acl/destinations', () => m.cmp_nav_item_destinations()],
    ['/acl/aliases', () => m.cmp_nav_item_aliases()],
    ['/activity', () => m.cmp_nav_item_activity_log()],
    ['/network-devices', () => m.cmp_nav_item_network_devices()],
    ['/openid', () => m.cmp_nav_item_openid()],
    ['/webhooks', () => m.cmp_nav_item_webhooks()],
    ['/settings', () => m.cmp_nav_item_settings()],
    ['/support', () => m.cmp_nav_item_support()],
    ['/edges', () => m.cmp_nav_item_edges()],
  ]);

  return _labelMap;
}

/**
 * Returns the translated navigation label for a given app route, or undefined
 * if the route does not correspond to a known navigation item.
 *
 * The route is canonicalized before lookup, so trailing slashes and leading
 * slash omissions are tolerated.
 *
 * @example
 *   getRouteLabel('/locations')   // → "Locations"
 *   getRouteLabel('/settings/')   // → "Settings"
 *   getRouteLabel('/unknown')     // → undefined
 */
export function getRouteLabel(route: string): string | undefined {
  const key = canonicalizeRouteKey(route);
  const labelFn = getLabelMap().get(key);
  return labelFn?.();
}
