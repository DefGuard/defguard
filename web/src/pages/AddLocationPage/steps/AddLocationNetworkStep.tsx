import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { AddLocationPageStep } from '../types';
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
      useAddLocationStore.setState({
        ...value,
        activeStep: AddLocationPageStep.Mfa,
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
                label="Keep alive interval (seconds)"
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="mtu">
            {(field) => (
              <field.FormInput label="Maximum Transmission Unit (MTU)" type="number" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="fwmark">
            {(field) => <field.FormInput label="Firewall Mark (FwMark)" type="number" />}
          </form.AppField>
          <ModalControls
            submitProps={{
              text: m.controls_continue(),
              testId: 'continue',
              onClick: () => {
                form.handleSubmit();
              },
            }}
          >
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
          </ModalControls>
        </form.AppForm>
      </form>
    </WizardCard>
  );
};
