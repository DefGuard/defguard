import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import { Button } from '../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/types';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { patternNoSpecialChars } from '../../../shared/patterns';
import { LoginData } from '../../../shared/types';

type Inputs = {
  username: string;
  password: string;
};

export const Login = () => {
  const { LL, locale } = useI18nContext();
  const schema = useMemo(
    () =>
      yup
        .object({
          username: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternNoSpecialChars, LL.form.error.noSpecialChars()),
          password: yup
            .string()
            .required(LL.form.error.required())
            .max(32, LL.form.error.maximumLength()),
        })
        .required(),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [locale],
  );

  const {
    auth: { login },
  } = useApi();

  const { handleSubmit, control, setError } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      password: '',
      username: '',
    },
  });

  const loginSubject = useAuthStore((state) => state.loginSubject);

  const loginMutation = useMutation((data: LoginData) => login(data), {
    mutationKey: [MutationKeys.LOG_IN],
    onSuccess: (data) => loginSubject.next(data),
    onError: (error: AxiosError) => {
      if (error.response && error.response.status === 401) {
        setError(
          'password',
          {
            message: 'username or password is incorrect',
          },
          { shouldFocus: true },
        );
      } else {
        console.error(error);
      }
    },
  });

  const onSubmit: SubmitHandler<Inputs> = (data) => {
    if (!loginMutation.isLoading) {
      loginMutation.mutate(data);
    }
  };

  return (
    <section id="login-container">
      <h1>{LL.loginPage.pageTitle()}</h1>
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormInput
          controller={{ control, name: 'username' }}
          placeholder={LL.form.placeholders.username()}
          autoComplete="username"
          data-testid="login-form-username"
          innerLabel
          required
        />
        <FormInput
          controller={{ control, name: 'password' }}
          placeholder={LL.form.placeholders.password()}
          type="password"
          autoComplete="password"
          data-testid="login-form-password"
          innerLabel
          required
        />
        <Button
          type="submit"
          loading={loginMutation.isLoading}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.form.login()}
          data-testid="login-form-submit"
        />
      </form>
    </section>
  );
};
