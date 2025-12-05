import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { validateIpList } from '../../../shared/validators';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const formSchema = z.object({
  name: z.string(m.form_error_required()).min(1, m.form_error_required()),
  address: z
    .string(m.form_error_required())
    .trim()
    .min(1, m.form_error_required())
    .refine((value) => validateIpList(value, ',', false), m.form_error_invalid()),
  port: z.number(m.form_error_required()).max(65535, m.form_error_port_max()),
});

type FormFields = z.infer<typeof formSchema>;

export const AddLocationStartStep = () => {
  const defaultValues = useAddLocationStore(
    useShallow(
      (s): FormFields => ({
        address: s.address,
        name: s.name,
        port: s.port,
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
        activeStep: AddLocationPageStep.InternalVpnSettings,
      });
    },
  });

  return (
    <WizardCard id="add-location-start-step">
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="name">
            {(field) => <field.FormInput required label={'Location name'} />}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title="Gateway address">
            <p>
              {
                'The VPN network will be derived from this address (e.g., 10.10.10.1 → 10.10.10.0). You can specify multiple addresses separated by commas. The first one is used as the primary address for device IP assignment.'
              }
            </p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="address">
            {(field) => <field.FormInput required label={'Gateway VPN IP address'} />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="port">
            {(field) => <field.FormInput required label={'Gateway port'} type="number" />}
          </form.AppField>
          <ModalControls
            submitProps={{
              text: m.controls_continue(),
              testId: 'continue',
              onClick: () => {
                form.handleSubmit();
              },
            }}
          />
        </form.AppForm>
      </form>
    </WizardCard>
  );
};
