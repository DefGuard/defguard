import z from 'zod';
import { m } from '../../../paraglide/messages';
import { LoginPage } from '../../../shared/components/LoginPage/LoginPage';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSize, ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import './style.scss';
import { revalidateLogic } from '@tanstack/react-form';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useSearch } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { useEffect, useMemo, useRef, useState } from 'react';
import api from '../../../shared/api/api';
import { getAuthErrorText } from '../../../shared/api/apiErrorMessages';
import type { OpenIdAuthInfo, OpenIdProviderKindValue } from '../../../shared/api/types';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ExternalProviderButton } from '../../../shared/defguard-ui/components/ExternalProviderButton/ExternalProviderButton';
import type { ExternalProviderButtonProps } from '../../../shared/defguard-ui/components/ExternalProviderButton/types';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { createZodIssue } from '../../../shared/defguard-ui/utils/zod';
import { useTheme } from '../../../shared/hooks/theme/useTheme';
import { useAuth } from '../../../shared/hooks/useAuth';

const formSchema = z.object({
  username: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  password: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
});

type FormFields = z.infer<typeof formSchema>;

const defaults: FormFields = {
  username: '',
  password: '',
};

export const LoginMainPage = () => {
  const [tooManyAttempts, setTooManyAttempts] = useState(false);
  const attemptsTimeoutRef = useRef<number | null>(null);
  const { authError, authErrorDetail } = useSearch({ from: '/auth/login' });
  const navigate = useNavigate({ from: '/auth/login' });
  const [authErrorText, setAuthErrorText] = useState<string[] | null>(null);

  // authError/authErrorDetail arrive via URL search params (redirect after OpenID/Entra failure);
  // consume them into local state then strip from URL so refresh/back-nav doesn't re-show stale error.
  useEffect(() => {
    if (!isPresent(authError) && !isPresent(authErrorDetail)) return;
    setAuthErrorText(getAuthErrorText(authError, authErrorDetail));
    void navigate({ search: {}, replace: true });
  }, [authError, authErrorDetail, navigate]);

  const { data: openIdAuthInfo } = useQuery({
    queryFn: api.openid.authInfo,
    queryKey: ['openid', 'auth_info'],
    select: (resp) => resp.data,
    retry: false,
  });

  const form = useAppForm({
    defaultValues: defaults,
    validationLogic: revalidateLogic({
      mode: 'change',
      modeAfterSubmission: 'change',
    }),
    validators: {
      onChange: formSchema,
      onSubmit: formSchema,
    },
    onSubmit: async ({ value }) => {
      if (tooManyAttempts) return;
      try {
        const { data } = await mutateAsync(value);
        useAuth.getState().authSubject.next(data);
      } catch (_) {}
    },
  });

  const { mutateAsync } = useMutation({
    mutationFn: api.auth.login,
    onError: (error: AxiosError) => {
      const status = error.response?.status;
      if (isPresent(status)) {
        if (status === 401) {
          form.setErrorMap({
            onSubmit: {
              fields: {
                password: createZodIssue(m.login_error_invalid(), ['password']),
              },
            },
          });
        }
        if (status === 429) {
          setTooManyAttempts(true);
          const timeoutId = setTimeout(() => {
            setTooManyAttempts(false);
          }, 300_000);
          attemptsTimeoutRef.current = timeoutId;
        }
      }
    },
  });

  useEffect(() => {
    return () => {
      if (attemptsTimeoutRef.current !== null) {
        clearTimeout(attemptsTimeoutRef.current);
      }
    };
  }, []);

  return (
    <LoginPage>
      <h1>{m.login_main_title()}</h1>
      <h2>{m.login_main_subtitle()}</h2>
      <SizedBox height={ThemeSize.Xl3} />
      {isPresent(authErrorText) && (
        <>
          <InfoBanner variant="warning" text={authErrorText} icon="info-outlined" />
          <SizedBox height={ThemeSpacing.Xl2} />
        </>
      )}
      {tooManyAttempts && (
        <>
          <InfoBanner
            variant="warning"
            text={m.login_main_attempts_info()}
            icon="info-outlined"
          />
          <SizedBox height={ThemeSpacing.Xl2} />
        </>
      )}
      {isPresent(openIdAuthInfo) && <LoginWithExternalProvider {...openIdAuthInfo} />}
      <form.AppForm>
        <form
          id="login-main-form"
          onSubmit={(e) => {
            e.preventDefault();
            e.stopPropagation();
            form.handleSubmit();
          }}
        >
          <form.AppField name="username">
            {(field) => (
              <field.FormInput
                label={m.form_label_username()}
                helper={m.form_helper_username()}
                size="lg"
              />
            )}
          </form.AppField>
          <form.AppField name="password">
            {(field) => (
              <field.FormInput
                type="password"
                label={m.form_label_password()}
                helper={m.form_helper_password()}
                size="lg"
              />
            )}
          </form.AppField>
          <Button
            text={m.controls_sign_in()}
            type="submit"
            testId="sign-in"
            variant="primary"
            size="big"
            loading={form.state.isSubmitting}
            disabled={tooManyAttempts}
          />
          <p className="forgot">{m.login_main_forgot()}</p>
        </form>
      </form.AppForm>
    </LoginPage>
  );
};

const buttonProvider: Record<
  OpenIdProviderKindValue,
  ExternalProviderButtonProps['provider']
> = {
  Custom: 'custom',
  Google: 'google',
  JumpCloud: 'jumpcloud',
  Microsoft: 'microsoft',
  Okta: 'okta',
  Zitadel: 'custom',
};

const LoginWithExternalProvider = (data: OpenIdAuthInfo) => {
  const { theme } = useTheme();
  const text = useMemo(() => {
    if (data.button_display_name) {
      return m.login_main_sign_in_with({ provider: data.button_display_name });
    }
    return m.login_main_sign_in_with_external_provider();
  }, [data.button_display_name]);

  return (
    <div id="external-login">
      <a href={data.url} target="_self" rel="noopener noreferrer">
        <ExternalProviderButton
          variant="default"
          provider={buttonProvider[data.provider_kind]}
          theme={theme}
          text={text}
        />
      </a>
      <Divider text={m.misc_or()} spacing={ThemeSpacing.Xl2} />
    </div>
  );
};
