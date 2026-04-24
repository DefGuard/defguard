import { z } from 'zod';
import type { ContextualHelpMappings } from './types';

const faqSchema = z
  .object({
    question: z.string().min(1),
    answer: z.string().min(1),
  })
  .strip();

const docSchema = z
  .object({
    title: z.string().min(1),
    url: z.string().url(),
  })
  .strip();

const pageSchema = z
  .object({
    faqs: z.array(faqSchema).optional(),
    relatedDocs: z.array(docSchema).optional(),
    bestPractices: z.string().optional(),
  })
  .strip();

const versionEntrySchema = z.record(z.string(), pageSchema);

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

export function parseContextualHelp(raw: unknown): ContextualHelpMappings {
  const parsed = mappingsSchema.parse(raw);
  return parsed.versions;
}
