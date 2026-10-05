import type z from 'zod';
import { m } from '../../../paraglide/messages';
import { MAX_MTU, MIN_MTU } from '../../constants';
import { AppText } from '../../defguard-ui/components/AppText/AppText';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../defguard-ui/types';
import { withFieldGroup } from '../../form';

type ClientMtuValues = {
  client_mtu_enabled: boolean;
  client_mtu: number | null;
};

/** `fields` for forms using the same field names. */
export const clientMtuFieldNames = {
  client_mtu_enabled: 'client_mtu_enabled',
  client_mtu: 'client_mtu',
} as const;

/** `superRefine` check for a custom client MTU. */
export const refineClientMtu = (value: ClientMtuValues, context: z.RefinementCtx) => {
  if (!value.client_mtu_enabled) return;
  if (value.client_mtu === null) {
    context.addIssue({
      code: 'custom',
      path: ['client_mtu'],
      message: m.form_error_required(),
    });
  } else if (value.client_mtu < MIN_MTU || value.client_mtu > MAX_MTU) {
    context.addIssue({
      code: 'custom',
      path: ['client_mtu'],
      message: m.form_error_invalid(),
    });
  }
};

/** Client's own MTU, or one set for the location. */
export const ClientMtuFields = withFieldGroup({
  defaultValues: { client_mtu_enabled: false, client_mtu: null } as ClientMtuValues,
  render: ({ group }) => (
    <>
      <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
        {m.location_network_client_mtu_title()}
      </AppText>
      <SizedBox height={ThemeSpacing.Xs} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {m.location_network_client_mtu_description()}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <group.AppField name="client_mtu_enabled">
        {(field) => (
          <>
            <field.FormRadio
              value={false}
              text={m.location_network_client_mtu_option_client()}
            />
            <SizedBox height={ThemeSpacing.Md} />
            <field.FormRadio
              value={true}
              text={m.location_network_client_mtu_option_custom()}
            />
          </>
        )}
      </group.AppField>
      <group.Subscribe selector={(state) => state.values.client_mtu_enabled}>
        {(clientMtuEnabled) =>
          clientMtuEnabled && (
            <>
              <SizedBox height={ThemeSpacing.Lg} />
              <group.AppField name="client_mtu">
                {(field) => (
                  <field.FormInput
                    required
                    label={m.location_network_label_client_mtu()}
                    type="number"
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
