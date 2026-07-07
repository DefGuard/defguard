import { useMutation } from '@tanstack/react-query';
import type { AxiosError } from 'axios';
import type z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { getApiErrorMessage } from '../../../shared/api/apiErrorMessages';
import type { ApiError } from '../../../shared/api/types';
import { LoginPage } from '../../../shared/components/LoginPage/LoginPage';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useEffectOnce } from '../../../shared/defguard-ui/hooks/useEffectOnce';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { useAuth } from '../../../shared/hooks/useAuth';
import { totpCodeFormSchema } from '../../../shared/schema/totpCode';
import { MfaLinks } from '../shared/MfaLinks/MfaLinks';

const formSchema = totpCodeFormSchema;

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  code: '',
};

export const LoginEmail = () => {
  const { mutate: resend } = useMutation({
    mutationFn: api.auth.mfa.email.resend,
    onError: (error: AxiosError<ApiError>) => {
      const code = error.response?.data?.code;
      Snackbar.error(isPresent(code) ? getApiErrorMessage(code) : m.error_unknown());
    },
  });

  useEffectOnce(() => {
    resend();
  });

  const { mutateAsync } = useMutation({
    mutationFn: api.auth.mfa.email.verify,
    meta: {
      invalidate: [['me'], ['session-info']],
    },
    onSuccess: (response) => {
      useAuth.getState().authSubject.next(response.data);
    },
    onError: (e: AxiosError<ApiError>) => {
      const respCode = e.response?.status;
      if (isPresent(respCode) && respCode < 500) {
        form.setErrorMap({
          onSubmit: {
            fields: {
              code: respCode === 429 ? m.login_main_attempts_info() : m.form_error_code(),
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
    <LoginPage id="login-email-page">
      <h1>{m.login_mfa_title()}</h1>
      <h2>{m.login_mfa_email_subtitle()}</h2>
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
              <field.FormInput
                notNull
                size="lg"
                label={m.form_label_auth_code()}
                helper={m.form_helper_auth_code()}
              />
            )}
          </form.AppField>
          <form.FormSubmitButton size="big" text={m.controls_submit()} />
        </form.AppForm>
      </form>
      <MfaLinks />
    </LoginPage>
  );
};
