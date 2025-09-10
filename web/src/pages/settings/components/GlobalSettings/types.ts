import z from 'zod';
import type { TranslationFunctions } from '../../../../i18n/i18n-types';

export const globalSettingsSchema = (LL: TranslationFunctions) =>
  z.object({
    main_logo_url: z.string().trim(),
    nav_logo_url: z.string().trim(),
    instance_name: z
      .string()
      .trim()
      .min(3, LL.form.error.minimumLength())
      .max(64, LL.form.error.maximumLength()),
    openid_enabled: z.boolean(),
    wireguard_enabled: z.boolean(),
    worker_enabled: z.boolean(),
    webhooks_enabled: z.boolean(),
    license: z.string().trim().optional(),
  });

export type GlobalSettingsFormFields = z.infer<ReturnType<typeof globalSettingsSchema>>;
