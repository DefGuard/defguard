import { z } from 'zod';
import { canonicalizeRouteKey } from './route-key';
import type { VideoSupportMappings } from './types';

// ---------------------------------------------------------------------------
// Source resolution
// ---------------------------------------------------------------------------

/**
 * Path (relative to the update service base URL) to load the video support
 * mapping from. Resolved by Vite at build time.
 *
 * The shared updateServiceClient resolves this against its baseURL
 * (VITE_UPDATE_BASE_URL ?? 'https://pkgs.defguard.net/api'), so the
 * production URL is https://pkgs.defguard.net/api/content/video-support.
 *
 * Override VITE_VIDEO_SUPPORT_URL to use a different path on the same server,
 * or VITE_UPDATE_BASE_URL to redirect all update-service calls to a local
 * server for development.
 */
export const videoSupportPath: string =
  import.meta.env.VITE_VIDEO_SUPPORT_URL ?? '/content/video-support';

// ---------------------------------------------------------------------------
// Zod schema + parser
// ---------------------------------------------------------------------------

const videoSupportSchema = z
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
  // The schema enforces leading "/" as a contract requirement for JSON authors.
  // canonicalizeRouteKey() adds it at runtime for widget use, but the JSON
  // must supply it explicitly — keeping authoring intent unambiguous.
  z.string().regex(/^\//, 'route key must start with "/"'),
  z.array(videoSupportSchema),
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
 * Validates raw JSON against the video support mapping contract and returns a
 * trusted VideoSupportMappings object with canonicalized route keys.
 * Throws a ZodError if the contract is violated.
 */
export function parseVideoSupport(raw: unknown): VideoSupportMappings {
  const parsed = mappingsSchema.parse(raw);

  const result: VideoSupportMappings = {};

  for (const [versionKey, routeMap] of Object.entries(parsed.versions)) {
    const canonicalRouteMap: Record<string, VideoSupportMappings[string][string]> = {};

    for (const [routeKey, videos] of Object.entries(routeMap)) {
      const canonical = canonicalizeRouteKey(routeKey);
      if (canonical in canonicalRouteMap) {
        throw new Error(
          `Duplicate route key "${canonical}" in version "${versionKey}" after canonicalization`,
        );
      }
      canonicalRouteMap[canonical] = videos;
    }

    result[versionKey] = canonicalRouteMap;
  }

  return result;
}
