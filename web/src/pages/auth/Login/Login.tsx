import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormInput } from '../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { patternSafeUsernameCharacters } from '../../../shared/patterns';
import { LoginData } from '../../../shared/types';
import { trimObjectStrings } from '../../../shared/utils/trimObjectStrings';

type Inputs = {
  username: string;
  password: string;
};

export const Login = () => {
  const { LL } = useI18nContext();

  const zodSchema = useMemo(
    () =>
      z.object({
        username: z
          .string()
          .min(3)
          .max(64)
          .regex(patternSafeUsernameCharacters, LL.form.error.forbiddenCharacter()),
        password: z.string().max(32, LL.form.error.maximumLength()),
      }),
    [LL.form.error],
  );

  const {
    auth: { login },
  } = useApi();

  const { handleSubmit, control, setError } = useForm<Inputs>({
    resolver: zodResolver(zodSchema),
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
      loginMutation.mutate(trimObjectStrings(data));
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
          required
        />
        <FormInput
          controller={{ control, name: 'password' }}
          placeholder={LL.form.placeholders.password()}
          type="password"
          autoComplete="password"
          data-testid="login-form-password"
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
