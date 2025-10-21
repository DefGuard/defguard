import type z from 'zod';
import { m } from '../../../paraglide/messages';
import { LoginPage } from '../../../shared/components/LoginPage/LoginPage';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useAppForm } from '../../../shared/defguard-ui/form';
import { ThemeSize } from '../../../shared/defguard-ui/types';
import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import api from '../../../shared/api/api';
import type { ApiError } from '../../../shared/api/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { formChangeLogic } from '../../../shared/form';
import { useAuth } from '../../../shared/hooks/useAuth';
import { totpCodeFormSchema } from '../../../shared/schema/totpCode';
import { MfaLinks } from '../shared/MfaLinks/MfaLinks';

const formSchema = totpCodeFormSchema;

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  code: '',
};

export const TotpLogin = () => {
  const navigate = useNavigate();
  const { mutateAsync } = useMutation({
    mutationFn: api.auth.mfa.totp.verify,
    onSuccess: (response) => {
      useAuth.getState().setUser(response.data.user);
      navigate({
        to: '/user/$username',
        params: {
          username: response.data.user.username,
        },
      });
    },
    onError: (e: AxiosError<ApiError>) => {
      const respCode = e.response?.status;
      if (isPresent(respCode) && respCode < 500) {
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
    <LoginPage id="mfa-totp-page">
      <h1>{m.login_mfa_title()}</h1>
      <h2>{m.login_totp_subtitle()}</h2>
      <SizedBox height={ThemeSize.Xl5} />
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="code">
            {(field) => <field.FormInput label={m.form_label_auth_code()} />}
          </form.AppField>
          <form.FormSubmitButton text={m.controls_submit()} />
        </form.AppForm>
      </form>
      <MfaLinks />
    </LoginPage>
  );
};
