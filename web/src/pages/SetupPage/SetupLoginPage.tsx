import { revalidateLogic } from '@tanstack/react-form';
import { useNavigate } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { LoginPage } from '../../shared/components/LoginPage/LoginPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { InfoBanner } from '../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSize, ThemeSpacing } from '../../shared/defguard-ui/types';
import { createZodIssue } from '../../shared/defguard-ui/utils/zod';
import { useAppForm } from '../../shared/form';
import '../auth/LoginMain/style.scss';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';

const formSchema = z.object({
  username: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  password: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
});

type FormFields = z.infer<typeof formSchema>;

const defaults: FormFields = {
  username: '',
  password: '',
};

export const SetupLoginPage = () => {
  const navigate = useNavigate();
  const [tooManyAttempts, setTooManyAttempts] = useState(false);
  const attemptsTimeoutRef = useRef<number | null>(null);

  const { mutateAsync } = useMutation({
    mutationFn: api.initial_setup.login,
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
        await mutateAsync(value);
        navigate({ to: '/setup', replace: true });
      } catch (error) {
        const status = (error as AxiosError).response?.status;
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
      <h2>{m.initial_setup_login_subtitle()}</h2>
      <SizedBox height={ThemeSize.Xl3} />
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
            testId="sign-in"
            variant="primary"
            size="big"
            loading={form.state.isSubmitting}
          />
        </form>
      </form.AppForm>
    </LoginPage>
  );
};
