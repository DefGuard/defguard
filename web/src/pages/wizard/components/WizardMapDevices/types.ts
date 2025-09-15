import z from 'zod';
import type { TranslationFunctions } from '../../../../i18n/i18n-types';

export const wizardMapDevicesSchema = (LL: TranslationFunctions) =>
  z.object({
    devices: z.array(
      z.object({
        wireguard_ips: z.array(z.string().trim().min(1, LL.form.error.required())),
        user_id: z
          .number({
            invalid_type_error: LL.form.error.required(),
          })
          .min(1, LL.form.error.required()),
        wireguard_pubkey: z.string().trim().min(1, LL.form.error.required()),
        name: z.string().trim().min(1, LL.form.error.required()),
      }),
    ),
  });

export type WizardMapDevicesFormFields = z.infer<
  ReturnType<typeof wizardMapDevicesSchema>
>;
