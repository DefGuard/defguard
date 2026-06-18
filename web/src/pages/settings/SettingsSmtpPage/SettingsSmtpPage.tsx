import './style.scss';
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
import { Page } from '../../../shared/components/Page/Page';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
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
import {
  getLicenseInfoQueryOptions,
  getSettingsQueryOptions,
} from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
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
        {isPresent(settings) && <Content settings={settings} smtpEnabled={smtp} />}
      </SettingsLayout>
      <SendTestEmailModal />
    </Page>
  );
};

const detectActiveCard = (
  authentication: string,
  issuerUrl: string | null,
  smtpServer: string,
): SmtpAuthCardVariant | null => {
  if (!smtpServer) return null;
  if (authentication === SmtpAuthentication.None) return 'none';
  if (authentication === SmtpAuthentication.Login) return 'basic';
  if (authentication === SmtpAuthentication.XOAuth2) {
    if (isGoogleIssuerUrl(issuerUrl)) return 'google';
    if (isMicrosoftIssuerUrl(issuerUrl)) return 'microsoft';
    return 'custom';
  }
  return null;
};

const AUTH_CARDS: SmtpAuthCardVariant[] = ['none', 'basic', 'google', 'microsoft'];

const formSchema = z.object({
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
  smtp_oauth_tenant_id: z.string().trim().nullable(),
});

type FormFields = z.infer<typeof formSchema>;

const emptyValues: FormFields = {
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
  smtp_oauth_tenant_id: null,
};

const Content = ({
  settings,
  smtpEnabled,
}: {
  settings: Settings;
  smtpEnabled: boolean;
}) => {
  const [modalVariant, setModalVariant] = useState<SmtpAuthCardVariant | null>(null);
  const modalInitialValuesRef = useRef<SmtpAuthModalValues>({
    smtp_server: '',
    smtp_port: 587,
    smtp_sender: '',
    smtp_encryption: SmtpEncryption.StartTls,
    smtp_user: null,
    smtp_password: null,
    smtp_oauth_issuer_url: null,
    smtp_oauth_client_id: null,
    smtp_oauth_client_secret: null,
    smtp_oauth_refresh_token: null,
    smtp_oauth_tenant_id: null,
  });

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
      smtp_oauth_tenant_id: settings.smtp_oauth_tenant_id ?? null,
    }),
    [settings],
  );

  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const oauthLocked =
    licenseInfo !== undefined && !canUseBusinessFeature(licenseInfo).result;

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
      onChange: formSchema,
    },
    onSubmit: async () => {},
  });

  const openConfigModal = (variant: SmtpAuthCardVariant) => {
    modalInitialValuesRef.current = {
      smtp_server: form.state.values.smtp_server,
      smtp_port: form.state.values.smtp_port,
      smtp_sender: form.state.values.smtp_sender,
      smtp_encryption: form.state.values.smtp_encryption as SmtpEncryptionValue,
      smtp_user: form.state.values.smtp_user,
      smtp_password: form.state.values.smtp_password,
      smtp_oauth_issuer_url: form.state.values.smtp_oauth_issuer_url,
      smtp_oauth_client_id: form.state.values.smtp_oauth_client_id,
      smtp_oauth_client_secret: form.state.values.smtp_oauth_client_secret,
      smtp_oauth_refresh_token: form.state.values.smtp_oauth_refresh_token,
      smtp_oauth_tenant_id: form.state.values.smtp_oauth_tenant_id,
    };
    setModalVariant(variant);
  };

  const openConfigModalGated = (variant: SmtpAuthCardVariant) => {
    if (variant === 'google' || variant === 'microsoft') {
      if (licenseInfo === undefined) return;
      licenseActionCheck(canUseBusinessFeature(licenseInfo), () =>
        openConfigModal(variant),
      );
    } else {
      openConfigModal(variant);
    }
  };

  const handleDelete = () => {
    openModal(ModalName.ConfirmAction, {
      title: m.settings_smtp_reset_confirm_title(),
      contentMd: m.settings_smtp_reset_confirm_body(),
      actionPromise: () => {
        return api.settings.patchSettings(emptyValues);
      },
      invalidateKeys: [['settings'], ['info']],
      submitProps: { text: m.controls_delete(), variant: 'critical' },
      onSuccess: () => {
        form.reset(emptyValues);
        Snackbar.default(m.settings_smtp_reset_success());
      },
      onError: () => Snackbar.error(m.settings_smtp_reset_failed()),
    });
  };

  return (
    <form.AppForm>
      <form.Subscribe
        selector={(s) => ({
          authentication: s.values.smtp_authentication,
          issuerUrl: s.values.smtp_oauth_issuer_url,
          smtpServer: s.values.smtp_server,
        })}
      >
        {({ authentication, issuerUrl, smtpServer }) => {
          const activeCard = smtpEnabled
            ? detectActiveCard(authentication, issuerUrl, smtpServer)
            : null;
          const otherCards = AUTH_CARDS.filter((v) => v !== activeCard);
          const renderCard = (variant: SmtpAuthCardVariant) => (
            <SmtpAuthMethodCard
              key={variant}
              variant={variant}
              active={activeCard === variant}
              locked={oauthLocked && (variant === 'google' || variant === 'microsoft')}
              onConfigure={() => openConfigModalGated(variant)}
              onEdit={() => openConfigModalGated(variant)}
              onSendTestEmail={() => openModal(ModalName.SendTestMail)}
              onDelete={handleDelete}
            />
          );
          if (!activeCard) {
            return (
              <div className="smtp-auth-method-cards">{AUTH_CARDS.map(renderCard)}</div>
            );
          }
          return (
            <>
              <p className="smtp-cards-section-label">
                {m.settings_smtp_active_config_label()}
              </p>
              {renderCard(activeCard)}
              <SizedBox height={ThemeSpacing.Xl2} />
              <p className="smtp-cards-section-label">
                {m.settings_smtp_other_methods_label()}
              </p>
              <div className="smtp-auth-method-cards">{otherCards.map(renderCard)}</div>
            </>
          );
        }}
      </form.Subscribe>
      <SmtpAuthConfigModal
        isOpen={modalVariant !== null}
        variant={modalVariant}
        initialValues={modalInitialValuesRef.current}
        onApply={async (result: SmtpAuthApplyResult) => {
          const cur = form.state.values;
          const merged: FormFields = {
            smtp_authentication: result.authentication,
            smtp_sender: result.smtp_sender,
            smtp_server: result.smtp_server ?? cur.smtp_server,
            smtp_port: result.smtp_port ?? cur.smtp_port,
            smtp_encryption: result.smtp_encryption ?? cur.smtp_encryption,
            smtp_user: result.smtp_user !== undefined ? result.smtp_user : cur.smtp_user,
            smtp_password:
              result.smtp_password !== undefined
                ? result.smtp_password
                : cur.smtp_password,
            smtp_oauth_issuer_url:
              result.smtp_oauth_issuer_url !== undefined
                ? result.smtp_oauth_issuer_url
                : cur.smtp_oauth_issuer_url,
            smtp_oauth_client_id:
              result.smtp_oauth_client_id !== undefined
                ? result.smtp_oauth_client_id
                : cur.smtp_oauth_client_id,
            smtp_oauth_client_secret:
              result.smtp_oauth_client_secret !== undefined
                ? result.smtp_oauth_client_secret
                : cur.smtp_oauth_client_secret,
            smtp_oauth_refresh_token:
              result.smtp_oauth_refresh_token !== undefined
                ? result.smtp_oauth_refresh_token
                : cur.smtp_oauth_refresh_token,
            smtp_oauth_tenant_id:
              result.smtp_oauth_tenant_id !== undefined
                ? result.smtp_oauth_tenant_id
                : cur.smtp_oauth_tenant_id,
          };
          if (merged.smtp_authentication !== SmtpAuthentication.Login) {
            merged.smtp_user = null;
            merged.smtp_password = null;
          }
          if (merged.smtp_authentication !== SmtpAuthentication.XOAuth2) {
            merged.smtp_oauth_issuer_url = null;
            merged.smtp_oauth_client_id = null;
            merged.smtp_oauth_client_secret = null;
            merged.smtp_oauth_refresh_token = null;
            merged.smtp_oauth_tenant_id = null;
          }
          const currentActiveCard = detectActiveCard(
            cur.smtp_authentication,
            cur.smtp_oauth_issuer_url,
            cur.smtp_server,
          );
          if (currentActiveCard !== null && modalVariant !== currentActiveCard) {
            openModal(ModalName.ConfirmAction, {
              title: m.settings_smtp_activate_confirm_title(),
              contentMd: m.settings_smtp_activate_confirm_body(),
              actionPromise: () => editSettings(merged),
              submitProps: { text: m.controls_continue() },
              onSuccess: () => {
                form.reset(merged);
                setModalVariant(null);
              },
            });
            return;
          }
          await editSettings(merged);
          form.reset(merged);
          setModalVariant(null);
        }}
        onClose={() => setModalVariant(null)}
      />
    </form.AppForm>
  );
};
