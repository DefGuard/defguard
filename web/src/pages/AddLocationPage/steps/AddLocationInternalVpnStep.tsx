import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
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
  endpoint: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  allowed_ips: z.string(m.form_error_required()).trim(),
  dns: z.string().nullable(),
});

type FormFields = z.infer<typeof formSchema>;

export const AddLocationInternalVpnStep = () => {
  const defaultValues = useAddLocationStore(
    useShallow(
      (s): FormFields => ({
        allowed_ips: s.allowed_ips,
        dns: s.dns,
        endpoint: s.endpoint,
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
        activeStep: AddLocationPageStep.NetworkSettings,
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
          <form.AppField name="endpoint">
            {(field) => (
              <field.FormInput required label="Gateway VPN IP address and netmask" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title={`Allowed IP's`}>
            <p>{`List of addresses/masks that should be routed through the VPN network.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="allowed_ips">
            {(field) => <field.FormInput required label={'Allowed IPs'} />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="dns">
            {(field) => <field.FormInput label={'DNS'} />}
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
                  activeStep: AddLocationPageStep.Start,
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
