import { useMutation, useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useMemo, useRef, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import {
  type Settings,
  SmtpAuthentication,
  SmtpEncryption,
  type SmtpEncryptionValue,
} from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import {
  ContextualHelpKey,
  ContextualHelpSidebar,
} from '../../../shared/components/ContextualHelp';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
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
import {
  type SmtpAuthCardVariant,
  SmtpAuthMethodCard,
} from './components/SmtpAuthMethodCard/SmtpAuthMethodCard';
import { SendTestEmailModal } from './SendTestEmailModal';
import {
  type SmtpAuthApplyResult,
  SmtpAuthConfigModal,
  type SmtpAuthModalValues,
} from './SmtpAuthConfigModal';
import { isGoogleIssuerUrl, isMicrosoftIssuerUrl } from './smtpAuthUtils';

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
      <SettingsLayout
        suggestion={<ContextualHelpSidebar pageKey={ContextualHelpKey.SettingsSmtp} />}
      >
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

const detectActiveCard = (
  authentication: string,
  issuerUrl: string | null,
): SmtpAuthCardVariant | null => {
  if (authentication === SmtpAuthentication.Login) return 'basic';
  if (authentication === SmtpAuthentication.XOAuth2) {
    if (isGoogleIssuerUrl(issuerUrl)) return 'google';
    if (isMicrosoftIssuerUrl(issuerUrl)) return 'microsoft';
    return 'custom';
  }
  return null;
};

const AUTH_CARDS: SmtpAuthCardVariant[] = ['basic', 'custom', 'google', 'microsoft'];

const Content = ({ settings }: { settings: Settings }) => {
  const smtpConfigured = useApp((s) => s.appInfo.smtp_enabled);
  const [modalVariant, setModalVariant] = useState<SmtpAuthCardVariant | null>(null);
  const modalInitialValuesRef = useRef<SmtpAuthModalValues>({
    smtp_user: null,
    smtp_password: null,
    smtp_oauth_issuer_url: null,
    smtp_oauth_client_id: null,
    smtp_oauth_client_secret: null,
    smtp_oauth_refresh_token: null,
  });
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
        smtp_password: z.string().trim().nullable(),
        smtp_user: z.string().trim().nullable(),
        smtp_sender: z
          .string()
          .trim()
          .min(1, m.form_error_required())
          .regex(patternValidEmail, m.form_error_email()),
        smtp_encryption: z.enum(SmtpEncryption),
        smtp_authentication: z.enum(SmtpAuthentication),
        smtp_oauth_issuer_url: z.string().trim().nullable(),
        smtp_oauth_client_id: z.string().trim().nullable(),
        smtp_oauth_client_secret: z.string().trim().nullable(),
        smtp_oauth_refresh_token: z.string().trim().nullable(),
        use_auth: z.boolean(),
      }),
    [],
  );

  type FormFields = z.infer<typeof formSchema>;

  const emptyValues = useMemo(
    (): FormFields => ({
      smtp_encryption: SmtpEncryption.StartTls,
      smtp_password: null,
      smtp_port: 587,
      smtp_sender: '',
      smtp_server: '',
      smtp_user: null,
      smtp_authentication: SmtpAuthentication.None,
      smtp_oauth_issuer_url: null,
      smtp_oauth_client_id: null,
      smtp_oauth_client_secret: null,
      smtp_oauth_refresh_token: null,
      use_auth: false,
    }),
    [],
  );

  const defaultValues = useMemo(
    (): FormFields => ({
      smtp_encryption: settings.smtp_encryption,
      smtp_password: settings.smtp_password ?? null,
      smtp_port: settings.smtp_port ?? 587,
      smtp_sender: settings.smtp_sender ?? '',
      smtp_server: settings.smtp_server ?? '',
      smtp_user: settings.smtp_user ?? null,
      smtp_authentication: settings.smtp_authentication,
      smtp_oauth_issuer_url: settings.smtp_oauth_issuer_url ?? null,
      smtp_oauth_client_id: settings.smtp_oauth_client_id ?? null,
      smtp_oauth_client_secret: settings.smtp_oauth_client_secret ?? null,
      smtp_oauth_refresh_token: settings.smtp_oauth_refresh_token ?? null,
      use_auth: settings.smtp_authentication !== SmtpAuthentication.None,
    }),
    [settings],
  );

  const { mutateAsync: editSettings } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: [['settings'], ['info']],
    },
    onSuccess: () => {
      Snackbar.default(m.settings_msg_saved());
    },
    onError: () => {
      Snackbar.error(m.settings_msg_save_failed());
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
      const { use_auth, ...rest } = value;
      let submitValue = { ...rest };
      if (!use_auth) {
        submitValue = {
          ...submitValue,
          smtp_authentication: SmtpAuthentication.None,
          smtp_user: null,
          smtp_password: null,
          smtp_oauth_issuer_url: null,
          smtp_oauth_client_id: null,
          smtp_oauth_client_secret: null,
          smtp_oauth_refresh_token: null,
        };
      } else {
        if (submitValue.smtp_authentication !== SmtpAuthentication.Login) {
          submitValue.smtp_user = null;
          submitValue.smtp_password = null;
        }
        if (submitValue.smtp_authentication !== SmtpAuthentication.XOAuth2) {
          submitValue.smtp_oauth_issuer_url = null;
          submitValue.smtp_oauth_client_id = null;
          submitValue.smtp_oauth_client_secret = null;
          submitValue.smtp_oauth_refresh_token = null;
        }
      }
      await editSettings(submitValue);
      form.reset(value);
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
        <form.Subscribe
          selector={(s) => ({
            authentication: s.values.smtp_authentication,
            issuerUrl: s.values.smtp_oauth_issuer_url,
            useAuth: s.values.use_auth,
          })}
        >
          {({ authentication, issuerUrl, useAuth }) => {
            const activeCard = useAuth
              ? detectActiveCard(authentication, issuerUrl)
              : null;
            const disableServerPort = activeCard === 'google';
            const disableEncryption =
              activeCard === 'google' || activeCard === 'microsoft';
            return (
              <>
                <EvenSplit>
                  <form.AppField name="smtp_server">
                    {(field) => (
                      <field.FormInput
                        required
                        disabled={disableServerPort}
                        label={m.settings_smtp_label_server_address()}
                        helper={m.settings_smtp_helper_server_address()}
                      />
                    )}
                  </form.AppField>
                  <form.AppField name="smtp_port">
                    {(field) => (
                      <field.FormInput
                        required
                        disabled={disableServerPort}
                        label={m.settings_smtp_label_server_port()}
                        helper={m.settings_smtp_helper_server_port()}
                        type="number"
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
                        disabled={disableEncryption}
                      />
                    )}
                  </form.AppField>
                </EvenSplit>
              </>
            );
          }}
        </form.Subscribe>
        <Divider spacing={ThemeSpacing.Xl2} />
        <DescriptionBlock title={m.settings_smtp_section_auth_title()}>
          <p>{m.settings_smtp_section_auth_description()}</p>
        </DescriptionBlock>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="use_auth">
          {(field) => (
            <>
              <field.FormRadio value={false} text={m.settings_smtp_auth_option_none()} />
              <SizedBox height={ThemeSpacing.Md} />
              <field.FormRadio
                value={true}
                text={m.settings_smtp_auth_option_enabled()}
              />
            </>
          )}
        </form.AppField>
        <form.Subscribe
          selector={(s) => ({
            useAuth: s.values.use_auth,
            authentication: s.values.smtp_authentication,
            issuerUrl: s.values.smtp_oauth_issuer_url,
          })}
        >
          {({ useAuth, authentication, issuerUrl }) => {
            if (!useAuth) return null;
            const activeCard = detectActiveCard(authentication, issuerUrl);
            return (
              <>
                <SizedBox height={ThemeSpacing.Xl} />
                <div className="smtp-auth-method-cards">
                  {AUTH_CARDS.map((variant) => (
                    <SmtpAuthMethodCard
                      key={variant}
                      variant={variant}
                      active={activeCard === variant}
                      onApply={() => {
                        modalInitialValuesRef.current = {
                          smtp_user: form.state.values.smtp_user,
                          smtp_password: form.state.values.smtp_password,
                          smtp_oauth_issuer_url: form.state.values.smtp_oauth_issuer_url,
                          smtp_oauth_client_id: form.state.values.smtp_oauth_client_id,
                          smtp_oauth_client_secret:
                            form.state.values.smtp_oauth_client_secret,
                          smtp_oauth_refresh_token:
                            form.state.values.smtp_oauth_refresh_token,
                        };
                        setModalVariant(variant);
                      }}
                    />
                  ))}
                </div>
              </>
            );
          }}
        </form.Subscribe>
        <SmtpAuthConfigModal
          isOpen={modalVariant !== null}
          variant={modalVariant}
          initialValues={modalInitialValuesRef.current}
          onApply={({
            authentication,
            values,
            smtp_server,
            smtp_port,
            smtp_encryption,
          }: SmtpAuthApplyResult) => {
            form.setFieldValue('smtp_authentication', authentication);
            form.setFieldValue('smtp_user', values.smtp_user);
            form.setFieldValue('smtp_password', values.smtp_password);
            form.setFieldValue('smtp_oauth_issuer_url', values.smtp_oauth_issuer_url);
            form.setFieldValue('smtp_oauth_client_id', values.smtp_oauth_client_id);
            form.setFieldValue(
              'smtp_oauth_client_secret',
              values.smtp_oauth_client_secret,
            );
            form.setFieldValue(
              'smtp_oauth_refresh_token',
              values.smtp_oauth_refresh_token,
            );
            if (smtp_server !== undefined) {
              form.setFieldValue('smtp_server', smtp_server);
            }
            if (smtp_port !== undefined) {
              form.setFieldValue('smtp_port', smtp_port);
            }
            if (smtp_encryption !== undefined) {
              form.setFieldValue('smtp_encryption', smtp_encryption);
            }
            setModalVariant(null);
          }}
          onClose={() => setModalVariant(null)}
        />
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
                      actionPromise: () => {
                        const { use_auth: _, ...resetValues } = emptyValues;
                        return api.settings.patchSettings(resetValues);
                      },
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
