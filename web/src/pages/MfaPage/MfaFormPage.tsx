import { Link, useNavigate } from '@tanstack/react-router';
import z from 'zod';
import { m } from '../../paraglide/messages';
import { Controls } from '../../shared/components/Controls/Controls';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { MfaConfiguration } from '../../shared/components/MfaConfiguration/MfaConfiguration';
import {
  type MfaConfigurationStepData,
  MfaMethod,
} from '../../shared/components/MfaConfiguration/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';

const formSchema = z.object({
  name: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  steps: z
    .array(
      z.object({
        id: z.union([z.string(), z.number()]),
        methods: z.array(z.enum(MfaMethod)).min(1),
      }),
    )
    .min(1, m.mfa_flow_step_required()),
});

export const MfaFormPage = () => {
  const navigate = useNavigate();
  const form = useAppForm({
    defaultValues: {
      name: '',
      steps: [] as MfaConfigurationStepData[],
    },
    validationLogic: formChangeLogic,
    validators: {
      onChange: formSchema,
      onSubmit: formSchema,
    },
  });

  return (
    <EditPage
      pageTitle={m.cmp_nav_item_mfa()}
      links={[
        <Link key="mfa-flows" to="/mfa">
          {m.cmp_nav_item_mfa()}
        </Link>,
        <Link key="create-mfa-flow" to="/mfa/add-flow">
          {m.mfa_flow_breadcrumb_create()}
        </Link>,
      ]}
      onBack={() => navigate({ to: '/mfa' })}
      headerProps={{
        icon: 'activity-notes',
        title: m.mfa_flow_form_title_create(),
        subtitle: m.mfa_flow_form_subtitle(),
      }}
    >
      <form.AppForm>
        <MarkedSection icon="settings">
          <MarkedSectionHeader title={m.mfa_flow_form_general_settings()} />
          <form.AppField name="name">
            {(field) => <field.FormInput required label={m.mfa_flow_form_name()} />}
          </form.AppField>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="manage-keys">
          <MarkedSectionHeader
            title={m.mfa_flow_form_methods_title()}
            description={m.mfa_flow_form_methods_description()}
          />
          <form.AppField name="steps">
            {(field) => {
              const error = field.state.meta.errors[0];
              return (
                <MfaConfiguration
                  steps={field.state.value}
                  onChange={field.handleChange}
                  error={typeof error === 'string' ? error : error?.message}
                />
              );
            }}
          </form.AppField>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <Controls>
          <div className="right">
            <Button type="button" variant="secondary" text={m.controls_cancel()} />
            <Button type="button" text={m.mfa_flow_form_action_create()} />
          </div>
        </Controls>
      </form.AppForm>
    </EditPage>
  );
};
