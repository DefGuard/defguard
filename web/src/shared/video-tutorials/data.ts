import { z } from 'zod';
import type { VideoTutorialsMappings } from './types';

// ---------------------------------------------------------------------------
// Source resolution
// ---------------------------------------------------------------------------

/**
 * Path (relative to the update service base URL) to load the video tutorials
 * mapping from. Resolved by Vite at build time.
 *
 * The shared updateServiceClient resolves this against its baseURL
 * (VITE_UPDATE_BASE_URL ?? 'https://pkgs.defguard.net/api'), so the
 * production URL is https://pkgs.defguard.net/api/content/video-tutorials.
 *
 * Override VITE_VIDEO_TUTORIALS_URL to use a different path on the same server,
 * or VITE_UPDATE_BASE_URL to redirect all update-service calls to a local
 * server for development.
 */
export const videoTutorialsPath: string =
  import.meta.env.VITE_VIDEO_TUTORIALS_URL ?? '/content/video-tutorials';

// ---------------------------------------------------------------------------
// Zod schema + parser
// ---------------------------------------------------------------------------

const videoTutorialSchema = z
  .object({
    youtubeVideoId: z
      .string()
      .regex(
        /^[A-Za-z0-9_-]{11}$/,
        'youtubeVideoId must be exactly 11 alphanumeric/-/_ chars',
      ),
    title: z.string().min(1, 'title must be non-empty'),
    description: z.string().min(1, 'description must be non-empty'),
    appRoute: z.string().regex(/^\//, 'appRoute must start with "/"'),
    docsUrl: z.string().url('docsUrl must be a valid URL'),
  })
  .strip();

const sectionSchema = z
  .object({
    name: z.string().min(1, 'section name must be non-empty'),
    videos: z.array(videoTutorialSchema),
  })
  .strip();

const migrationWizardPlacementSchema = z
  .object({
    youtubeVideoId: z
      .string()
      .regex(
        /^[A-Za-z0-9_-]{11}$/,
        'youtubeVideoId must be exactly 11 alphanumeric/-/_ chars',
      ),
    title: z.string().min(1, 'title must be non-empty'),
    docsUrl: z.string().url('docsUrl must be a valid URL'),
  })
  .strip();

const placementsSchema = z
  .object({
    migrationWizard: migrationWizardPlacementSchema.optional(),
  })
  .strip();

const versionEntrySchema = z
  .object({
    sections: z.array(sectionSchema),
    placements: placementsSchema.optional(),
  })
  .strip();

const mappingsSchema = z.object({
  versions: z.record(
    z
      .string()
      .regex(
        /^\d+\.\d+(\.\d+)?$/,
        'version key must be major.minor or major.minor.patch',
      ),
    versionEntrySchema,
  ),
});

/**
 * Validates raw JSON against the video tutorials mapping contract and returns a
 * trusted VideoTutorialsMappings object.
 * Throws a ZodError if the contract is violated.
 */
export function parseVideoTutorials(raw: unknown): VideoTutorialsMappings {
  const parsed = mappingsSchema.parse(raw);
  return parsed.versions;
}
