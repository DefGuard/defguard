import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useEffect, useMemo } from 'react';
import { useForm } from 'react-hook-form';
import { SubmitHandler } from 'react-hook-form';
import { useNavigate } from 'react-router';
import * as yup from 'yup';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { patternNoSpecialChars } from '../../../shared/patterns';
import { LoginData, UserMFAMethod } from '../../../shared/types';
import { useMFAStore } from '../shared/hooks/useMFAStore';

type Inputs = {
  username: string;
  password: string;
};

const Login = () => {
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
    [locale]
  );
  const {
    auth: { login },
  } = useApi();
  const logIn = useAuthStore((state) => state.logIn);
  const navigate = useNavigate();
  const toaster = useToaster();

  const { handleSubmit, control, setError } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      password: '',
      username: '',
    },
  });

  const setMfaStore = useMFAStore((state) => state.setState);

  const loginMutation = useMutation((data: LoginData) => login(data), {
    mutationKey: [MutationKeys.LOG_IN],
    onSuccess: (data) => {
      const { url, user, mfa } = data;
      if (user && url) {
        logIn(user);
        window.location.replace(url);
        return;
      }
      if (!user && !mfa) {
        toaster.error(LL.messages.error());
        console.error('API returned unexpected result upon login.');
      } else {
        if (user) {
          logIn(user);
        }
        if (mfa) {
          setMfaStore(mfa);
          switch (mfa.mfa_method) {
            case UserMFAMethod.WEB3:
              navigate('../mfa/web3');
              break;
            case UserMFAMethod.WEB_AUTH_N:
              navigate('../mfa/webauthn');
              break;
            case UserMFAMethod.ONE_TIME_PASSWORD:
              navigate('../mfa/totp');
              break;
            default:
              toaster.error(LL.messages.error());
              console.error('API returned unexpected result upon login.');
              break;
          }
        }
      }
    },
    onError: (error: AxiosError) => {
      if (error.response && error.response.status === 401) {
        setError(
          'password',
          {
            message: 'username or password is incorrect',
          },
          { shouldFocus: true }
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

  useEffect(() => {
    setMfaStore({});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <section id="login-container">
      <h1>{LL.loginPage.pageTitle()}</h1>
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormInput
          controller={{ control, name: 'username' }}
          placeholder={LL.form.placeholders.username()}
          innerLabel
          required
        />
        <FormInput
          controller={{ control, name: 'password' }}
          placeholder={LL.form.placeholders.password()}
          type="password"
          innerLabel
          required
        />
        <Button
          type="submit"
          loading={loginMutation.isLoading}
          disabled={loginMutation.isLoading}
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.form.submit()}
        />
      </form>
    </section>
  );
};

export default Login;
