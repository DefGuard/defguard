import { queryOptions } from '@tanstack/react-query';
import axios from 'axios';
import { z } from 'zod';
import { canonicalizeRouteKey } from './route-key';
import type { HelpTutorialsMappings } from './types';

// ---------------------------------------------------------------------------
// Source resolution
// ---------------------------------------------------------------------------

/**
 * Returns the URL to load the tutorial mapping from.
 * Override via VITE_HELP_TUTORIALS_URL for remote API migration.
 */
export function resolveSource(): string {
  return import.meta.env.VITE_HELP_TUTORIALS_URL ?? '/help-tutorials.json';
}

// ---------------------------------------------------------------------------
// Fetch layer
// ---------------------------------------------------------------------------

/**
 * Fetches raw (unvalidated) data from the given URL.
 * Replace this function when migrating to an authenticated remote API.
 */
export async function fetchRawData(url: string): Promise<unknown> {
  const response = await axios.get<unknown>(url);
  return response.data;
}

// ---------------------------------------------------------------------------
// Zod schema + parser
// ---------------------------------------------------------------------------

const tutorialSchema = z
  .object({
    youtubeVideoId: z
      .string()
      .regex(
        /^[A-Za-z0-9_-]{11}$/,
        'youtubeVideoId must be exactly 11 alphanumeric/-/_ chars',
      ),
    title: z.string().min(1, 'title must be non-empty'),
  })
  .strip();

const routeMapSchema = z.record(
  z.string().regex(/^\//, 'route key must start with "/"'),
  z.array(tutorialSchema),
);

const mappingsSchema = z.object({
  versions: z.record(
    z
      .string()
      .regex(
        /^\d+\.\d+(\.\d+)?$/,
        'version key must be major.minor or major.minor.patch',
      ),
    routeMapSchema,
  ),
});

/**
 * Validates raw JSON against the tutorial mapping contract and returns a
 * trusted HelpTutorialsMappings object with canonicalized route keys.
 * Throws a ZodError if the contract is violated.
 */
export function parseHelpTutorials(raw: unknown): HelpTutorialsMappings {
  const parsed = mappingsSchema.parse(raw);

  const result: HelpTutorialsMappings = {};

  for (const [versionKey, routeMap] of Object.entries(parsed.versions)) {
    const canonicalRouteMap: Record<string, HelpTutorialsMappings[string][string]> = {};

    for (const [routeKey, tutorials] of Object.entries(routeMap)) {
      const canonical = canonicalizeRouteKey(routeKey);
      if (canonical in canonicalRouteMap) {
        throw new Error(
          `Duplicate route key "${canonical}" in version "${versionKey}" after canonicalization`,
        );
      }
      canonicalRouteMap[canonical] = tutorials;
    }

    result[versionKey] = canonicalRouteMap;
  }

  return result;
}

// ---------------------------------------------------------------------------
// React Query integration
// ---------------------------------------------------------------------------

export const helpTutorialsQueryOptions = queryOptions({
  queryKey: ['help-tutorials'],
  queryFn: () => fetchRawData(resolveSource()).then(parseHelpTutorials),
  // Tutorial mappings don't change at runtime — fetch once per session.
  // When migrating to a remote API, change this to an appropriate cache window.
  staleTime: Infinity,
  // Silent failure: if the fetch or parse fails, the widget simply won't appear.
  retry: false,
});
