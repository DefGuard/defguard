import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { AddLocationPageStep, type AddLocationPageStepValue } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const formSchema = z.object({
  keepalive_interval: z
    .number(m.form_error_required())
    .max(65535, m.form_error_port_max()),
  mtu: z.number(m.form_error_required()).min(72).max(0xffffffff),
  fwmark: z.number(m.form_error_required()).min(0).max(0xffffffff),
});

type FormFields = z.infer<typeof formSchema>;

export const AddLocationNetworkStep = () => {
  const locationType = useAddLocationStore((s) => s.locationType);

  const defaultValues = useAddLocationStore(
    useShallow(
      (s): FormFields => ({
        keepalive_interval: s.keepalive_interval,
        mtu: s.mtu,
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
        ...value,
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
                label={m.location_network_label_mtu()}
                helper={m.location_network_helper_mtu()}
                type="number"
              />
            )}
          </form.AppField>
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
                  ...form.state.values,
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
