import { Link, useNavigate } from '@tanstack/react-router';
import z from 'zod';
import { m } from '../../paraglide/messages';
import { Controls } from '../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../shared/components/DescriptionBlock/DescriptionBlock';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';

const formSchema = z.object({
  name: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
});

export const MfaFormPage = () => {
  const navigate = useNavigate();
  const form = useAppForm({
    defaultValues: {
      name: '',
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
          <DescriptionBlock title={m.mfa_flow_form_general_settings()}>
            <SizedBox height={ThemeSpacing.Lg} />
            <form.AppField name="name">
              {(field) => <field.FormInput required label={m.mfa_flow_form_name()} />}
            </form.AppField>
          </DescriptionBlock>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="manage-keys">
          <DescriptionBlock title={m.mfa_flow_form_methods_title()}>
            <SizedBox height={ThemeSpacing.Xs} />
            <p>{m.mfa_flow_form_methods_description()}</p>
          </DescriptionBlock>
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
