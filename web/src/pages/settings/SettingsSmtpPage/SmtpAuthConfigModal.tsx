import './style.scss';
import { useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import { SMTP_OAUTH_CALLBACK_TYPE } from '../../../routes/smtp-oauth-callback';
import type {
  SmtpAuthenticationValue,
  SmtpEncryptionValue,
} from '../../../shared/api/types';
import { SmtpAuthentication, SmtpEncryption } from '../../../shared/api/types';
import { EvenSplit } from '../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { Modal } from '../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import type { SmtpAuthCardVariant } from './components/SmtpAuthMethodCard/SmtpAuthMethodCard';

export type SmtpAuthModalValues = {
  smtp_user: string | null;
  smtp_password: string | null;
  smtp_oauth_issuer_url: string | null;
  smtp_oauth_client_id: string | null;
  smtp_oauth_client_secret: string | null;
  smtp_oauth_refresh_token: string | null;
};

export type SmtpAuthApplyResult = {
  authentication: SmtpAuthenticationValue;
  values: SmtpAuthModalValues;
  smtp_server?: string;
  smtp_port?: number;
  smtp_encryption?: SmtpEncryptionValue;
};

type Props = {
  isOpen: boolean;
  variant: SmtpAuthCardVariant | null;
  initialValues: SmtpAuthModalValues;
  onApply: (result: SmtpAuthApplyResult) => void;
  onClose: () => void;
};

const GOOGLE_ISSUER_URL = 'https://accounts.google.com';
const MICROSOFT_ISSUER_URL = 'https://login.microsoftonline.com/common';
const GOOGLE_AUTH_URL = `${GOOGLE_ISSUER_URL}/o/oauth2/v2/auth`;
const GOOGLE_TOKEN_URL = 'https://oauth2.googleapis.com/token';
const MICROSOFT_AUTH_URL = `${MICROSOFT_ISSUER_URL}/oauth2/v2.0/authorize`;
const MICROSOFT_TOKEN_URL = `${MICROSOFT_ISSUER_URL}/oauth2/v2.0/token`;
const CUSTOM_SCOPE_DEFAULT = 'openid offline_access';
const GOOGLE_SMTP_SERVER = 'smtp.gmail.com';
const MICROSOFT_SMTP_SERVER = 'smtp.office365.com';
const PROVIDER_SMTP_PORT = 587;

const modalTitles: Record<SmtpAuthCardVariant, () => string> = {
  basic: m.settings_smtp_auth_modal_basic_title,
  custom: m.settings_smtp_auth_modal_custom_title,
  google: m.settings_smtp_auth_modal_google_title,
  microsoft: m.settings_smtp_auth_modal_microsoft_title,
};

const discoverOidcEndpoints = async (
  issuerUrl: string,
): Promise<{ authorizationEndpoint: string; tokenEndpoint: string }> => {
  const discoveryUrl = `${issuerUrl.replace(/\/$/, '')}/.well-known/openid-configuration`;
  const response = await fetch(discoveryUrl);
  if (!response.ok) {
    throw new Error(m.settings_smtp_auth_oauth_error());
  }
  const data = (await response.json()) as {
    authorization_endpoint?: string;
    token_endpoint?: string;
  };
  if (!data.authorization_endpoint || !data.token_endpoint) {
    throw new Error(m.settings_smtp_auth_oauth_error());
  }
  return {
    authorizationEndpoint: data.authorization_endpoint,
    tokenEndpoint: data.token_endpoint,
  };
};

const buildAuthUrl = (
  authorizationEndpoint: string,
  clientId: string,
  redirectUri: string,
  scope: string,
  extraParams?: Record<string, string>,
): string => {
  const params = new URLSearchParams({
    client_id: clientId,
    redirect_uri: redirectUri,
    response_type: 'code',
    scope,
    prompt: 'consent',
    ...extraParams,
  });
  return `${authorizationEndpoint}?${params.toString()}`;
};

const waitForOAuthCode = (popup: Window): Promise<string> =>
  new Promise((resolve, reject) => {
    const messageHandler = (event: MessageEvent) => {
      if (event.origin !== window.location.origin) return;
      const data = event.data as {
        type?: string;
        code?: string;
        error?: string;
      };
      if (data?.type !== SMTP_OAUTH_CALLBACK_TYPE) return;
      window.removeEventListener('message', messageHandler);
      clearInterval(pollInterval);
      if (data.code) {
        resolve(data.code);
      } else {
        reject(new Error(data.error ?? m.settings_smtp_auth_oauth_error()));
      }
    };

    const pollInterval = setInterval(() => {
      if (popup.closed) {
        window.removeEventListener('message', messageHandler);
        clearInterval(pollInterval);
        reject(new Error(m.settings_smtp_auth_oauth_popup_closed()));
      }
    }, 500);

    window.addEventListener('message', messageHandler);
  });

const exchangeCodeForToken = async (
  tokenUrl: string,
  code: string,
  clientId: string,
  clientSecret: string,
  redirectUri: string,
): Promise<string> => {
  const body = new URLSearchParams({
    code,
    client_id: clientId,
    client_secret: clientSecret,
    redirect_uri: redirectUri,
    grant_type: 'authorization_code',
  });
  const response = await fetch(tokenUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body.toString(),
  });
  if (!response.ok) {
    const error = (await response.json().catch(() => ({}))) as {
      error_description?: string;
    };
    throw new Error(error.error_description ?? m.settings_smtp_auth_oauth_error());
  }
  const data = (await response.json()) as { refresh_token?: string };
  if (!data.refresh_token) {
    throw new Error(m.settings_smtp_auth_oauth_error());
  }
  return data.refresh_token;
};

export const SmtpAuthConfigModal = ({
  isOpen,
  variant,
  initialValues,
  onApply,
  onClose,
}: Props) => {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={variant ? modalTitles[variant]() : ''}
      size="primary"
    >
      {variant && (
        <ModalContent
          key={variant}
          variant={variant}
          initialValues={initialValues}
          onApply={onApply}
          onClose={onClose}
        />
      )}
    </Modal>
  );
};

const formSchema = z.object({
  smtp_user: z.string().trim().nullable(),
  smtp_password: z.string().trim().nullable(),
  smtp_oauth_issuer_url: z.string().trim().nullable(),
  smtp_oauth_scope: z.string().trim().nullable(),
  smtp_oauth_client_id: z.string().trim().nullable(),
  smtp_oauth_client_secret: z.string().trim().nullable(),
});

type ModalContentProps = {
  variant: SmtpAuthCardVariant;
  initialValues: SmtpAuthModalValues;
  onApply: (result: SmtpAuthApplyResult) => void;
  onClose: () => void;
};

const ModalContent = ({
  variant,
  initialValues,
  onApply,
  onClose,
}: ModalContentProps) => {
  const [oauthError, setOauthError] = useState<string | null>(null);

  const defaultValues = useMemo((): z.infer<typeof formSchema> => {
    switch (variant) {
      case 'basic':
        return {
          smtp_user: initialValues.smtp_user,
          smtp_password: initialValues.smtp_password,
          smtp_oauth_issuer_url: null,
          smtp_oauth_scope: null,
          smtp_oauth_client_id: null,
          smtp_oauth_client_secret: null,
        };
      case 'custom':
        return {
          smtp_user: null,
          smtp_password: null,
          smtp_oauth_issuer_url: initialValues.smtp_oauth_issuer_url,
          smtp_oauth_scope: CUSTOM_SCOPE_DEFAULT,
          smtp_oauth_client_id: initialValues.smtp_oauth_client_id,
          smtp_oauth_client_secret: initialValues.smtp_oauth_client_secret,
        };
      case 'google':
        return {
          smtp_user: null,
          smtp_password: null,
          smtp_oauth_issuer_url: GOOGLE_ISSUER_URL,
          smtp_oauth_scope: null,
          smtp_oauth_client_id: initialValues.smtp_oauth_client_id,
          smtp_oauth_client_secret: initialValues.smtp_oauth_client_secret,
        };
      case 'microsoft':
        return {
          smtp_user: null,
          smtp_password: null,
          smtp_oauth_issuer_url: initialValues.smtp_oauth_issuer_url?.includes(
            'microsoftonline',
          )
            ? initialValues.smtp_oauth_issuer_url
            : MICROSOFT_ISSUER_URL,
          smtp_oauth_scope: null,
          smtp_oauth_client_id: initialValues.smtp_oauth_client_id,
          smtp_oauth_client_secret: initialValues.smtp_oauth_client_secret,
        };
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    variant,
    initialValues.smtp_password,
    initialValues.smtp_oauth_client_secret,
    initialValues.smtp_user,
    initialValues.smtp_oauth_issuer_url?.includes,
    initialValues.smtp_oauth_issuer_url,
    initialValues.smtp_oauth_client_id,
  ]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      setOauthError(null);
      const redirectUri = `${window.location.origin}/smtp-oauth-callback`;

      if (variant === 'custom') {
        let authorizationEndpoint: string;
        let tokenEndpoint: string;
        try {
          const endpoints = await discoverOidcEndpoints(
            value.smtp_oauth_issuer_url ?? '',
          );
          authorizationEndpoint = endpoints.authorizationEndpoint;
          tokenEndpoint = endpoints.tokenEndpoint;
        } catch (err) {
          setOauthError(
            err instanceof Error ? err.message : m.settings_smtp_auth_oauth_error(),
          );
          return;
        }

        const authUrl = buildAuthUrl(
          authorizationEndpoint,
          value.smtp_oauth_client_id ?? '',
          redirectUri,
          value.smtp_oauth_scope ?? CUSTOM_SCOPE_DEFAULT,
        );

        const popup = window.open(
          authUrl,
          'smtp-oauth',
          'width=600,height=700,noopener=no',
        );
        if (!popup) {
          setOauthError(m.settings_smtp_auth_oauth_popup_blocked());
          return;
        }

        try {
          const code = await waitForOAuthCode(popup);
          const refreshToken = await exchangeCodeForToken(
            tokenEndpoint,
            code,
            value.smtp_oauth_client_id ?? '',
            value.smtp_oauth_client_secret ?? '',
            redirectUri,
          );
          onApply({
            authentication: SmtpAuthentication.XOAuth2,
            values: {
              ...value,
              smtp_oauth_refresh_token: refreshToken,
            },
          });
        } catch (err) {
          setOauthError(
            err instanceof Error ? err.message : m.settings_smtp_auth_oauth_error(),
          );
        }
        return;
      }

      if (variant === 'google' || variant === 'microsoft') {
        const [authorizationEndpoint, tokenEndpoint] =
          variant === 'google'
            ? [GOOGLE_AUTH_URL, GOOGLE_TOKEN_URL]
            : [MICROSOFT_AUTH_URL, MICROSOFT_TOKEN_URL];

        const [scope, extraParams] =
          variant === 'google'
            ? (['https://mail.google.com/ email', { access_type: 'offline' }] as const)
            : (['https://outlook.office.com/SMTP.Send offline_access', {}] as const);

        const authUrl = buildAuthUrl(
          authorizationEndpoint,
          value.smtp_oauth_client_id ?? '',
          redirectUri,
          scope,
          extraParams,
        );

        const popup = window.open(
          authUrl,
          'smtp-oauth',
          'width=600,height=700,noopener=no',
        );
        if (!popup) {
          setOauthError(m.settings_smtp_auth_oauth_popup_blocked());
          return;
        }

        try {
          const code = await waitForOAuthCode(popup);
          const refreshToken = await exchangeCodeForToken(
            tokenEndpoint,
            code,
            value.smtp_oauth_client_id ?? '',
            value.smtp_oauth_client_secret ?? '',
            redirectUri,
          );
          onApply({
            authentication: SmtpAuthentication.XOAuth2,
            values: {
              ...value,
              smtp_oauth_refresh_token: refreshToken,
            },
            smtp_server:
              variant === 'google' ? GOOGLE_SMTP_SERVER : MICROSOFT_SMTP_SERVER,
            smtp_port: PROVIDER_SMTP_PORT,
            smtp_encryption: SmtpEncryption.StartTls,
          });
        } catch (err) {
          setOauthError(
            err instanceof Error ? err.message : m.settings_smtp_auth_oauth_error(),
          );
        }
        return;
      }

      onApply({
        authentication: SmtpAuthentication.Login,
        values: { ...value, smtp_oauth_refresh_token: null },
      });
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
        {variant === 'basic' && (
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
        )}

        {variant === 'custom' && (
          <>
            <form.AppField name="smtp_oauth_issuer_url">
              {(field) => (
                <field.FormInput
                  label={m.settings_smtp_label_oauth_issuer_url()}
                  helper={m.settings_smtp_helper_oauth_issuer_url()}
                />
              )}
            </form.AppField>
            <SizedBox height={ThemeSpacing.Xl} />
            <form.AppField name="smtp_oauth_scope">
              {(field) => (
                <field.FormInput
                  label={m.settings_smtp_label_oauth_scope()}
                  helper={m.settings_smtp_helper_oauth_scope()}
                />
              )}
            </form.AppField>
            <SizedBox height={ThemeSpacing.Xl} />
            <EvenSplit>
              <form.AppField name="smtp_oauth_client_id">
                {(field) => (
                  <field.FormInput
                    label={m.settings_smtp_label_oauth_client_id()}
                    helper={m.settings_smtp_helper_oauth_client_id()}
                  />
                )}
              </form.AppField>
              <form.AppField name="smtp_oauth_client_secret">
                {(field) => (
                  <field.FormInput
                    label={m.settings_smtp_label_oauth_client_secret()}
                    helper={m.settings_smtp_helper_oauth_client_secret()}
                    type="password"
                  />
                )}
              </form.AppField>
            </EvenSplit>
            <SizedBox height={ThemeSpacing.Md} />
            <p className="smtp-auth-oauth-info">{m.settings_smtp_auth_oauth_info()}</p>
            {oauthError && <p className="smtp-auth-oauth-error">{oauthError}</p>}
          </>
        )}

        {(variant === 'google' || variant === 'microsoft') && (
          <>
            <EvenSplit>
              <form.AppField name="smtp_oauth_client_id">
                {(field) => (
                  <field.FormInput
                    label={m.settings_smtp_label_oauth_client_id()}
                    helper={m.settings_smtp_helper_oauth_client_id()}
                  />
                )}
              </form.AppField>
              <form.AppField name="smtp_oauth_client_secret">
                {(field) => (
                  <field.FormInput
                    label={m.settings_smtp_label_oauth_client_secret()}
                    helper={m.settings_smtp_helper_oauth_client_secret()}
                    type="password"
                  />
                )}
              </form.AppField>
            </EvenSplit>
            <SizedBox height={ThemeSpacing.Md} />
            <p className="smtp-auth-oauth-info">{m.settings_smtp_auth_oauth_info()}</p>
            {oauthError && <p className="smtp-auth-oauth-error">{oauthError}</p>}
          </>
        )}

        <SizedBox height={ThemeSpacing.Xl2} />
        <form.Subscribe selector={(s) => ({ isSubmitting: s.isSubmitting })}>
          {({ isSubmitting }) => (
            <ModalControls
              submitProps={{
                text: m.controls_apply(),
                loading: isSubmitting,
                onClick: () => form.handleSubmit(),
              }}
              cancelProps={{
                text: m.controls_cancel(),
                disabled: isSubmitting,
                onClick: onClose,
              }}
            />
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
