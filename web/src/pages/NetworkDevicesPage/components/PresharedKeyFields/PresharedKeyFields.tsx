import z from 'zod';
import { m } from '../../../../paraglide/messages';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { withFieldGroup } from '../../../../shared/form';
import { patternValidWireguardKey } from '../../../../shared/patterns';

type PresharedKeyValues = {
  generatePresharedKey: boolean;
  preshared_key?: string | null;
};

export const presharedKeyFieldNames = {
  generatePresharedKey: 'generatePresharedKey',
  preshared_key: 'preshared_key',
} as const;

export const refinePresharedKey = (
  value: PresharedKeyValues,
  context: z.RefinementCtx,
) => {
  if (value.generatePresharedKey) return;
  const result = z
    .string(m.form_error_required())
    .regex(patternValidWireguardKey, m.form_error_invalid_preshared_key())
    .safeParse(value.preshared_key);
  if (!result.success) {
    context.addIssue({
      code: 'custom',
      message: result.error.issues[0].message,
      path: ['preshared_key'],
    });
  }
};

export const PresharedKeyFields = withFieldGroup({
  defaultValues: {
    generatePresharedKey: true,
    preshared_key: null,
  } as PresharedKeyValues,
  render: ({ group }) => (
    <>
      <group.AppField name="generatePresharedKey">
        {(field) => (
          <>
            <field.FormRadio
              value={true}
              text={m.modal_network_device_psk_choice_generate()}
            />
            <SizedBox height={ThemeSpacing.Md} />
            <field.FormRadio
              value={false}
              text={m.modal_network_device_psk_choice_manual()}
            />
          </>
        )}
      </group.AppField>
      <group.Subscribe selector={(state) => state.values.generatePresharedKey}>
        {(generatePresharedKey) =>
          !generatePresharedKey && (
            <>
              <SizedBox height={ThemeSpacing.Lg} />
              <group.AppField name="preshared_key">
                {(field) => (
                  <field.FormSecretInput
                    required
                    stored={false}
                    label={m.form_label_preshared_key()}
                  />
                )}
              </group.AppField>
            </>
          )
        }
      </group.Subscribe>
    </>
  ),
});
