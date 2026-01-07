import { z } from 'zod';

export const enterpriseUpgradeToastMetaSchema = z.object({
  customId: z.string().trim().min(1),
});

export type EnterpriseUpgradeToastMeta = z.infer<typeof enterpriseUpgradeToastMetaSchema>;
