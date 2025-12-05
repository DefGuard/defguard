import { useMutation } from '@tanstack/react-query';
import type { AxiosError } from 'axios';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { ApiError } from '../../../shared/api/types';
import { LoginPage } from '../../../shared/components/LoginPage/LoginPage';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { useAuth } from '../../../shared/hooks/useAuth';
import { MfaLinks } from '../shared/MfaLinks/MfaLinks';

const formSchema = z.object({
  code: z.string().trim().min(1, m.form_error_required()),
});

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  code: '',
};

export const LoginRecovery = () => {
  const { mutateAsync } = useMutation({
    mutationFn: api.auth.mfa.recovery,
    onSuccess: ({ data }) => {
      const user = data.user;
      useAuth.getState().setUser(user);
    },
    onError: (e: AxiosError<ApiError>) => {
      const code = e.response?.status;
      if (isPresent(code) && code < 500) {
        form.setErrorMap({
          onSubmit: {
            fields: {
              code: m.form_error_code(),
            },
          },
        });
      }
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
      await mutateAsync(value.code);
    },
  });

  return (
    <LoginPage id="mfa-recovery-page">
      <h1>{m.login_mfa_title()}</h1>
      <h2>{m.login_mfa_recovery_subtitle()}</h2>
      <SizedBox height={ThemeSpacing.Xl5} />
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="code">
            {(field) => (
              <field.FormInput size="lg" label={m.login_mfa_recovery_label()} />
            )}
          </form.AppField>
          <form.FormSubmitButton
            size="big"
            testId="submit-recovery-code"
            text={m.controls_submit()}
          />
        </form.AppForm>
      </form>
      <SizedBox height={ThemeSpacing.Xl5} />
      <MfaLinks />
    </LoginPage>
  );
};
