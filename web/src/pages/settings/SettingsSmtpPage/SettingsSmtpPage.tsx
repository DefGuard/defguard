import { useMutation, useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import {
  type Settings,
  SmtpEncryption,
  type SmtpEncryptionValue,
} from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { EvenSplit } from '../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import type { SelectOption } from '../../../shared/defguard-ui/components/Select/types';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { patternValidEmail } from '../../../shared/patterns';
import { getSettingsQueryOptions } from '../../../shared/query';
import { validateIpOrDomain } from '../../../shared/validators';

const breadcrumbsLinks = [
  <Link
    to="/settings"
    search={{
      tab: 'notifications',
    }}
    key={0}
  >
    Notifications
  </Link>,
  <Link key={1} to="/settings/smtp">
    SMTP Configuration
  </Link>,
];

export const SettingsSmtpPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);

  return (
    <Page id="settings-smtp-page" title="Settings">
      <Breadcrumbs links={breadcrumbsLinks} />
      <SettingsLayout>
        <SettingsHeader
          title="SMTP Configuration"
          subtitle="Here you can configure SMTP server used to send system messages to the users."
          icon="mail"
        />
        {isPresent(settings) && (
          <SettingsCard>
            <DescriptionBlock title="Server settings">
              <p>
                Configure the SMTP server here — it’s required for sending system messages
                to users.
              </p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Xl2} />
            <Content settings={settings} />
          </SettingsCard>
        )}
      </SettingsLayout>
    </Page>
  );
};

const encryptionValueToLabel = (value: SmtpEncryptionValue): string => {
  switch (value) {
    case 'ImplicitTls':
      return 'Implicit TLS';
    case 'StartTls':
      return 'Start TLS';
    case 'None':
      return 'None';
  }
};

const encryptionSelectOptions: SelectOption<SmtpEncryptionValue>[] = Object.values(
  SmtpEncryption,
).map((e) => ({
  key: e,
  label: encryptionValueToLabel(e),
  value: e,
}));

const Content = ({ settings }: { settings: Settings }) => {
  const formSchema = useMemo(
    () =>
      z.object({
        smtp_server: z
          .string()
          .trim()
          .min(1, m.form_error_required())
          .refine((val) => (!val ? true : validateIpOrDomain(val, false, true))),
        smtp_port: z.number(m.form_error_required()).max(65535, m.form_error_port_max()),
        smtp_password: z.string().trim(),
        smtp_user: z.string().trim(),
        smtp_sender: z
          .string()
          .trim()
          .min(1, m.form_error_required())
          .regex(patternValidEmail, m.form_error_email()),
        smtp_encryption: z.enum(SmtpEncryption),
      }),
    [],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      smtp_encryption: settings.smtp_encryption,
      smtp_password: settings.smtp_password ?? '',
      smtp_port: settings.smtp_port ?? 587,
      smtp_sender: settings.smtp_sender ?? '',
      smtp_server: settings.smtp_server ?? '',
      smtp_user: settings.smtp_user ?? '',
    }),
    [settings],
  );

  const { mutateAsync: editSettings } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: ['settings'],
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await editSettings(value);
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <EvenSplit>
          <form.AppField name="smtp_server">
            {(field) => <field.FormInput required label="Server Address" />}
          </form.AppField>
          <form.AppField name="smtp_port">
            {(field) => <field.FormInput required label="Server port" />}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl} />
        <EvenSplit>
          <form.AppField name="smtp_user">
            {(field) => <field.FormInput label="Server username" />}
          </form.AppField>
          <form.AppField name="smtp_password">
            {(field) => <field.FormInput label="Server password" type="password" />}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl} />
        <EvenSplit>
          <form.AppField name="smtp_sender">
            {(field) => <field.FormInput required label="Sender email address" />}
          </form.AppField>
          <form.AppField name="smtp_encryption">
            {(field) => (
              <field.FormSelect
                options={encryptionSelectOptions}
                label="Encryption"
                required
              />
            )}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl2} />
        <form.Subscribe
          selector={(s) => ({
            isDefaultValue: s.isDefaultValue || s.isPristine,
            isSubmitting: s.isSubmitting,
          })}
        >
          {({ isDefaultValue, isSubmitting }) => (
            <Controls>
              <div className="right">
                <Button
                  text={m.controls_save_changes()}
                  disabled={isDefaultValue}
                  loading={isSubmitting}
                  onClick={() => {
                    form.handleSubmit();
                  }}
                />
              </div>
            </Controls>
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
