import z from 'zod';
import type { TranslationFunctions } from '../../../../../i18n/i18n-types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { validateWireguardPublicKey } from '../../../../../shared/validators';
import { WGConfigGenChoice } from '../../AddStandaloneDeviceModal/types';
import { StandaloneDeviceModalFormMode } from '../types';

type SchemaProps = {
  mode: StandaloneDeviceModalFormMode;
  reservedNames: string[];
  originalName?: string;
};

export const standaloneDeviceFormSchema = (
  LL: TranslationFunctions,
  { mode, reservedNames, originalName }: SchemaProps,
) => {
  const errors = LL.form.error;

  return z
    .object({
      name: z
        .string()
        .min(1, LL.form.error.required())
        .refine((value) => {
          if (mode === StandaloneDeviceModalFormMode.EDIT && isPresent(originalName)) {
            const filtered = reservedNames.filter((n) => n !== originalName.trim());
            return !filtered.includes(value.trim());
          }
          return !reservedNames.includes(value.trim());
        }, LL.form.error.reservedName()),
      location_id: z.number(),
      description: z.string().optional(),
      modifiableIpParts: z.array(z.string().min(1, LL.form.error.required())),
      generationChoice: z.nativeEnum(WGConfigGenChoice),
      wireguard_pubkey: z.string().optional(),
    })
    .superRefine((vals, ctx) => {
      if (mode === StandaloneDeviceModalFormMode.CREATE_MANUAL) {
        if (vals.generationChoice === WGConfigGenChoice.MANUAL) {
          const result = validateWireguardPublicKey({
            requiredError: errors.required(),
            maxError: errors.maximumLengthOf({ length: 44 }),
            minError: errors.minimumLengthOf({ length: 44 }),
            validKeyError: errors.invalid(),
          }).safeParse(vals.wireguard_pubkey);
          if (!result.success) {
            result.error.errors.forEach((e) => {
              ctx.addIssue({
                path: ['wireguard_pubkey'],
                message: e.message,
                code: 'custom',
              });
            });
          }
        }
      }
    });
};

export type StandaloneDeviceFormFields = z.infer<
  ReturnType<typeof standaloneDeviceFormSchema>
>;
