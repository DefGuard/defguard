import { useState } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import { SmtpAuthentication } from '../../../../shared/api/types';
import { EvenSplit } from '../../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { FieldError } from '../../../../shared/defguard-ui/components/FieldError/FieldError';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { patternValidEmail } from '../../../../shared/patterns';
import {
  buildAuthUrl,
  CUSTOM_SCOPE_DEFAULT,
  discoverOidcEndpoints,
  exchangeCodeForToken,
  waitForOAuthCode,
} from './oauthFlow';
import type { FormProps } from './types';

const schema = z.object({
  smtp_sender: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .regex(patternValidEmail, m.form_error_email()),
  smtp_oauth_issuer_url: z.string().trim().nullable(),
  smtp_oauth_scope: z.string().trim(),
  smtp_oauth_client_id: z.string().trim().nullable(),
  smtp_oauth_client_secret: z.string().trim().nullable(),
});

export const CustomAuthForm = ({ initialValues, onApply, onClose }: FormProps) => {
  const [oauthError, setOauthError] = useState<string | null>(null);

  const form = useAppForm({
    defaultValues: {
      smtp_sender: initialValues.smtp_sender,
      smtp_oauth_issuer_url: initialValues.smtp_oauth_issuer_url,
      smtp_oauth_scope: CUSTOM_SCOPE_DEFAULT,
      smtp_oauth_client_id: initialValues.smtp_oauth_client_id,
      smtp_oauth_client_secret: initialValues.smtp_oauth_client_secret,
    },
    validationLogic: formChangeLogic,
    validators: { onSubmit: schema, onChange: schema },
    onSubmit: async ({ value }) => {
      setOauthError(null);
      const redirectUri = `${window.location.origin}/smtp-oauth-callback`;

      let authorizationEndpoint: string;
      let tokenEndpoint: string;
      try {
        const endpoints = await discoverOidcEndpoints(value.smtp_oauth_issuer_url ?? '');
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
        value.smtp_oauth_scope,
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
        await onApply({
          authentication: SmtpAuthentication.XOAuth2,
          smtp_sender: value.smtp_sender,
          smtp_oauth_issuer_url: value.smtp_oauth_issuer_url,
          smtp_oauth_client_id: value.smtp_oauth_client_id,
          smtp_oauth_client_secret: value.smtp_oauth_client_secret,
          smtp_oauth_refresh_token: refreshToken,
        });
      } catch (err) {
        setOauthError(
          err instanceof Error ? err.message : m.settings_smtp_auth_oauth_error(),
        );
      }
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
          <form.AppField name="smtp_sender">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_sender_email_address()}
                helper={m.settings_smtp_helper_sender_email_address()}
              />
            )}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl} />
        <EvenSplit>
          <form.AppField name="smtp_oauth_client_id">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_oauth_client_id()}
                helper={m.settings_smtp_helper_oauth_client_id()}
              />
            )}
          </form.AppField>
          <form.AppField name="smtp_oauth_client_secret">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_oauth_client_secret()}
                helper={m.settings_smtp_helper_oauth_client_secret()}
                type="password"
              />
            )}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="smtp_oauth_issuer_url">
          {(field) => (
            <field.FormInput
              required
              label={m.settings_smtp_label_oauth_issuer_url()}
              helper={m.settings_smtp_helper_oauth_issuer_url()}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="smtp_oauth_scope">
          {(field) => (
            <field.FormInput
              required
              label={m.settings_smtp_label_oauth_scope()}
              helper={m.settings_smtp_helper_oauth_scope()}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Md} />
        <p className="smtp-auth-oauth-info">{m.settings_smtp_auth_oauth_info()}</p>
        <FieldError error={oauthError} />
        <SizedBox height={ThemeSpacing.Xl2} />
        <form.Subscribe selector={(s) => ({ isSubmitting: s.isSubmitting })}>
          {({ isSubmitting }) => (
            <ModalControls
              submitProps={{
                text: m.controls_submit(),
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
