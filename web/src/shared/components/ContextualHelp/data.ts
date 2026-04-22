import { parseContextualHelp } from './schema';

/**
 * Path (relative to the update service base URL) from which the contextual
 * help mapping is loaded.
 *
 * Production URL: https://pkgs.defguard.net/api/content/contextual-help
 * Override VITE_CONTEXTUAL_HELP_URL to use a different path on the same server,
 * or VITE_UPDATE_BASE_URL to redirect all update-service calls to a local server.
 */
export const contextualHelpPath: string =
  import.meta.env.VITE_CONTEXTUAL_HELP_URL ?? '/content/contextual-help';

export { parseContextualHelp };
