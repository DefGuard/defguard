import { z } from 'zod';

export const versionUpdateToastMetaSchema = z.object({
  customId: z.string().min(1),
});

export type VersionUpdateToastMeta = z.infer<typeof versionUpdateToastMetaSchema>;
