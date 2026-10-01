import { useQuery } from '@tanstack/react-query';
import { omit } from 'lodash-es';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import type { SelectionOption } from '../../../shared/components/SelectionSection/type';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { getGroupsInfoQueryOptions } from '../../../shared/query';
import { LocationGroupMtuSection } from '../../EditLocationPage/components/LocationGroupMtuSection/LocationGroupMtuSection';
import { AddLocationPageStep, type AddLocationPageStepValue } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const formSchema = z
  .object({
    keepalive_interval: z
      .number(m.form_error_required())
      // Keepalive is mandatory to prevent idle service locations from disconnecting
      .min(1, m.form_error_keepalive_min())
      .max(65535, m.form_error_port_max()),
    mtu: z.number(m.form_error_required()).min(72).max(0xffffffff),
    client_mtu_enabled: z.boolean(),
    client_mtu: z.number().nullable(),
    fwmark: z.number(m.form_error_required()).min(0).max(0xffffffff),
  })
  .superRefine((value, context) => {
    if (value.client_mtu_enabled) {
      if (value.client_mtu === null) {
        context.addIssue({
          code: 'custom',
          path: ['client_mtu'],
          message: m.form_error_required(),
        });
      } else if (value.client_mtu < 72 || value.client_mtu > 0xffffffff) {
        context.addIssue({
          code: 'custom',
          path: ['client_mtu'],
          message: m.form_error_invalid(),
        });
      }
    }
  });

type FormFields = z.infer<typeof formSchema>;

// Drops the UI-only flag.
const toStoreValues = (value: FormFields) => ({
  ...omit(value, ['client_mtu_enabled']),
  client_mtu: value.client_mtu_enabled ? value.client_mtu : null,
});

export const AddLocationNetworkStep = () => {
  const locationType = useAddLocationStore((s) => s.locationType);
  const groupClientMtus = useAddLocationStore((s) => s.group_client_mtus);
  const { data: groupOptions = [] } = useQuery({
    ...getGroupsInfoQueryOptions,
    select: (response) =>
      response.data.map(
        (group): SelectionOption<number> => ({
          id: group.id,
          label: group.name,
        }),
      ),
  });

  const defaultValues = useAddLocationStore(
    useShallow(
      (s): FormFields => ({
        keepalive_interval: s.keepalive_interval,
        mtu: s.mtu,
        client_mtu_enabled: isPresent(s.client_mtu),
        client_mtu: s.client_mtu,
        fwmark: s.fwmark,
      }),
    ),
  );
  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: ({ value }) => {
      let targetStep: AddLocationPageStepValue;
      if (locationType === 'regular') {
        targetStep = AddLocationPageStep.Mfa;
      } else {
        targetStep = AddLocationPageStep.ServiceLocationSettings;
      }
      useAddLocationStore.setState({
        ...toStoreValues(value),
        activeStep: targetStep,
      });
    },
  });

  return (
    <WizardCard>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="keepalive_interval">
            {(field) => (
              <field.FormInput
                required
                label={m.location_network_label_keepalive_interval()}
                helper={m.location_network_helper_keepalive_interval()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="mtu">
            {(field) => (
              <field.FormInput
                required
                label={m.location_network_label_mtu()}
                helper={m.location_network_helper_mtu()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
            {m.location_network_client_mtu_title()}
          </AppText>
          <SizedBox height={ThemeSpacing.Xs} />
          <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
            {m.location_network_client_mtu_description()}
          </AppText>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="client_mtu_enabled">
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
          </form.AppField>
          <form.Subscribe selector={(state) => state.values.client_mtu_enabled}>
            {(clientMtuEnabled) =>
              clientMtuEnabled && (
                <>
                  <SizedBox height={ThemeSpacing.Lg} />
                  <form.AppField name="client_mtu">
                    {(field) => (
                      <field.FormInput
                        required
                        label={m.location_network_label_client_mtu()}
                        type="number"
                      />
                    )}
                  </form.AppField>
                </>
              )
            }
          </form.Subscribe>
          <SizedBox height={ThemeSpacing.Xl} />
          <LocationGroupMtuSection
            overrides={groupClientMtus}
            groupOptions={groupOptions}
            onChange={(group_client_mtus) =>
              useAddLocationStore.setState({ group_client_mtus })
            }
          />
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="fwmark">
            {(field) => (
              <field.FormInput
                label={m.location_network_label_fwmark()}
                helper={m.location_network_helper_fwmark()}
                type="number"
              />
            )}
          </form.AppField>
          <Controls>
            <Button
              variant="outlined"
              text={m.controls_back()}
              onClick={() => {
                useAddLocationStore.setState({
                  activeStep: AddLocationPageStep.InternalVpnSettings,
                  ...toStoreValues(form.state.values),
                });
              }}
            />
            <div className="right">
              <Button
                text={m.controls_continue()}
                testId="continue"
                onClick={() => {
                  form.handleSubmit();
                }}
              />
            </div>
          </Controls>
        </form.AppForm>
      </form>
    </WizardCard>
  );
};
