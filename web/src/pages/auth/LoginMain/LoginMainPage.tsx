import z from 'zod';
import { m } from '../../../paraglide/messages';
import { LoginPage } from '../../../shared/components/LoginPage/LoginPage';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useAppForm } from '../../../shared/defguard-ui/form';
import { ThemeSize, ThemeSpacing } from '../../../shared/defguard-ui/types';
import './style.scss';
import { revalidateLogic } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { useEffect, useRef, useState } from 'react';
import api from '../../../shared/api/api';
import type { LoginMfaResponse, LoginResponseBasic } from '../../../shared/api/types';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { createZodIssue } from '../../../shared/defguard-ui/utils/zod';
import { useAuth } from '../../../shared/hooks/useAuth';

const formSchema = z.object({
  username: z.string().trim().min(1, m.form_error_required()),
  password: z.string().trim().min(1, m.form_error_required()),
});

type FormFields = z.infer<typeof formSchema>;

const defaults: FormFields = {
  username: '',
  password: '',
};

export const LoginMainPage = () => {
  const [tooManyAttempts, setTooManyAttempts] = useState(false);
  const attemptsTimeoutRef = useRef<number | null>(null);
  const setAuthStore = useAuth((s) => s.setUser);
  const navigate = useNavigate();
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

        // @ts-expect-error
        // biome-ignore lint/complexity/useLiteralKeys: needed
        if (data['user'] !== undefined) {
          const basicResponse = data as LoginResponseBasic;
          setAuthStore(basicResponse.user);
          navigate({
            to: '/user/$username',
            params: {
              username: basicResponse.user?.username as string,
            },
          });
        } else {
          const mfa = data as LoginMfaResponse;
          useAuth.setState({
            mfaLogin: mfa,
          });
        }
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
      <SizedBox height={ThemeSize.Xl5} />
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
            {(field) => <field.FormInput label={m.form_label_username()} size="lg" />}
          </form.AppField>
          <form.AppField name="password">
            {(field) => (
              <field.FormInput
                type="password"
                label={m.form_label_password()}
                size="lg"
              />
            )}
          </form.AppField>
          <Button
            text="Sign in"
            type="submit"
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
