import { useMutation, useQuery } from '@tanstack/react-query';
import type z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { LoginPage } from '../../../shared/components/LoginPage/LoginPage';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
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
  useQuery({
    queryFn: api.auth.mfa.email.resend,
    queryKey: ['auth', 'email'],
    refetchOnWindowFocus: false,
    refetchOnReconnect: true,
    refetchOnMount: true,
  });

  const { mutateAsync } = useMutation({
    mutationFn: api.auth.mfa.email.verify,
    meta: {
      invalidate: ['me'],
    },
    onSuccess: (response) => {
      useAuth.getState().authSubject.next(response.data);
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
            {(field) => <field.FormInput size="lg" label={m.form_label_auth_code()} />}
          </form.AppField>
          <form.FormSubmitButton size="big" text={m.controls_submit()} />
        </form.AppForm>
      </form>
      <MfaLinks />
    </LoginPage>
  );
};
