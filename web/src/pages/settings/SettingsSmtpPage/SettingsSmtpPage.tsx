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
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../../shared/hooks/useApp';
import { patternValidEmail } from '../../../shared/patterns';
import { getSettingsQueryOptions } from '../../../shared/query';
import { Validate } from '../../../shared/validate';
import { getConfiguredBadge, getNotConfiguredBadge } from '../SettingsIndexPage/types';
import { SendTestEmailModal } from './SendTestEmailModal';

const breadcrumbsLinks = [
  <Link
    to="/settings"
    search={{
      tab: 'notifications',
    }}
    key={0}
  >
    {m.settings_breadcrumb_notifications()}
  </Link>,
  <Link key={1} to="/settings/smtp">
    {m.settings_smtp_title()}
  </Link>,
];

export const SettingsSmtpPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  const smtp = useApp((s) => s.appInfo.smtp_enabled);

  return (
    <Page id="settings-smtp-page" title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbsLinks} />
      <SettingsLayout>
        <SettingsHeader
          title={m.settings_smtp_title()}
          subtitle={m.settings_smtp_subtitle()}
          icon="mail"
          badgeProps={smtp ? getConfiguredBadge() : getNotConfiguredBadge()}
        />
        {isPresent(settings) && (
          <SettingsCard>
            <DescriptionBlock title={m.settings_smtp_section_server_title()}>
              <p>{m.settings_smtp_section_server_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Xl2} />
            <Content settings={settings} />
          </SettingsCard>
        )}
      </SettingsLayout>
      <SendTestEmailModal />
    </Page>
  );
};

const encryptionValueToLabel = (value: SmtpEncryptionValue): string => {
  switch (value) {
    case 'ImplicitTls':
      return m.settings_smtp_encryption_implicit_tls();
    case 'StartTls':
      return m.settings_smtp_encryption_start_tls();
    case 'None':
      return m.settings_smtp_encryption_none();
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
  const smtpConfigured = useApp((s) => s.appInfo.smtp_enabled);
  const formSchema = useMemo(
    () =>
      z.object({
        smtp_server: z
          .string()
          .trim()
          .min(1, m.form_error_required())
          .refine((val) =>
            !val
              ? true
              : Validate.any(
                  val,
                  [Validate.IPv4, Validate.IPv6, Validate.Domain, Validate.Hostname],
                  false,
                ),
          ),
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

  const emptyValues = useMemo(
    (): FormFields => ({
      smtp_encryption: SmtpEncryption.StartTls,
      smtp_password: '',
      smtp_port: 587,
      smtp_sender: '',
      smtp_server: '',
      smtp_user: '',
    }),
    [],
  );

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
      invalidate: [['settings'], ['info']],
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
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_server_address()}
                helper={m.settings_smtp_helper_server_address()}
              />
            )}
          </form.AppField>
          <form.AppField name="smtp_port">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_server_port()}
                helper={m.settings_smtp_helper_server_port()}
                type="number"
              />
            )}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl} />
        <EvenSplit>
          <form.AppField name="smtp_user">
            {(field) => (
              <field.FormInput
                label={m.settings_smtp_label_server_username()}
                helper={m.settings_smtp_helper_server_username()}
              />
            )}
          </form.AppField>
          <form.AppField name="smtp_password">
            {(field) => (
              <field.FormInput
                label={m.settings_smtp_label_server_password()}
                helper={m.settings_smtp_helper_server_password()}
                type="password"
              />
            )}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl} />
        <EvenSplit>
          <form.AppField name="smtp_sender">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_sender_email_address()}
                helper={m.settings_smtp_helper_sender_email_address()}
              />
            )}
          </form.AppField>
          <form.AppField name="smtp_encryption">
            {(field) => (
              <field.FormSelect
                options={encryptionSelectOptions}
                label={m.settings_smtp_label_encryption()}
                helper={m.settings_smtp_helper_encryption()}
                required
              />
            )}
          </form.AppField>
        </EvenSplit>
        <form.Subscribe
          selector={(s) => ({
            isDefaultValue: s.isDefaultValue || s.isPristine,
            isSubmitting: s.isSubmitting,
          })}
        >
          {({ isDefaultValue, isSubmitting }) => (
            <Controls>
              {smtpConfigured && (
                <Button
                  variant="critical"
                  text={m.settings_smtp_button_reset_settings()}
                  onClick={() => {
                    openModal(ModalName.ConfirmAction, {
                      title: m.settings_smtp_reset_confirm_title(),
                      contentMd: m.settings_smtp_reset_confirm_body(),
                      actionPromise: () => api.settings.patchSettings(emptyValues),
                      invalidateKeys: [['settings'], ['info']],
                      submitProps: { text: m.controls_reset(), variant: 'critical' },
                      onSuccess: () => {
                        form.reset(emptyValues);
                        Snackbar.default(m.settings_smtp_reset_success());
                      },
                      onError: () => Snackbar.error(m.settings_smtp_reset_failed()),
                    });
                  }}
                />
              )}
              <div className="right">
                {smtpConfigured && (
                  <Button
                    variant="outlined"
                    iconLeft="mail"
                    text={m.settings_smtp_button_send_test_email()}
                    onClick={() => {
                      openModal(ModalName.SendTestMail);
                    }}
                  />
                )}
                <Button
                  testId="save-changes"
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
