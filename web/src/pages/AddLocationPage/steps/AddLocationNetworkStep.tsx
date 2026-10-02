import { omit } from 'lodash-es';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import {
  ClientMtuFields,
  clientMtuFieldNames,
  refineClientMtu,
} from '../../../shared/components/LocationMtuSettings/ClientMtuFields';
import { LocationGroupMtuSection } from '../../../shared/components/LocationMtuSettings/LocationGroupMtuSection';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { MAX_MTU, MIN_MTU } from '../../../shared/constants';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { AddLocationPageStep, type AddLocationPageStepValue } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const formSchema = z
  .object({
    keepalive_interval: z
      .number(m.form_error_required())
      // Keepalive is mandatory to prevent idle service locations from disconnecting
      .min(1, m.form_error_keepalive_min())
      .max(65535, m.form_error_port_max()),
    mtu: z.number(m.form_error_required()).min(MIN_MTU).max(MAX_MTU),
    client_mtu_enabled: z.boolean(),
    client_mtu: z.number().nullable(),
    fwmark: z.number(m.form_error_required()).min(0).max(0xffffffff),
    peer_disconnect_threshold: z
      .number(m.form_error_required())
      .min(120, m.form_error_min({ value: 120 })),
  })
  .superRefine(refineClientMtu);

type FormFields = z.infer<typeof formSchema>;

// Drops the UI-only flag.
const toStoreValues = (value: FormFields) => ({
  ...omit(value, ['client_mtu_enabled']),
  client_mtu: value.client_mtu_enabled ? value.client_mtu : null,
  peer_disconnect_threshold: value.peer_disconnect_threshold,
});

export const AddLocationNetworkStep = () => {
  const locationType = useAddLocationStore((s) => s.locationType);
  const groupClientMtus = useAddLocationStore((s) => s.group_client_mtus);

  const defaultValues = useAddLocationStore(
    useShallow(
      (s): FormFields => ({
        keepalive_interval: s.keepalive_interval,
        mtu: s.mtu,
        client_mtu_enabled: isPresent(s.client_mtu),
        client_mtu: s.client_mtu,
        fwmark: s.fwmark,
        peer_disconnect_threshold: s.peer_disconnect_threshold,
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
          <ClientMtuFields form={form} fields={clientMtuFieldNames} />
          <SizedBox height={ThemeSpacing.Xl} />
          <LocationGroupMtuSection
            overrides={groupClientMtus}
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
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="peer_disconnect_threshold">
            {(field) => (
              <field.FormInput
                label={m.location_mfa_label_client_disconnect_threshold()}
                helper={m.location_mfa_helper_client_disconnect_threshold()}
                type="number"
                required
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
