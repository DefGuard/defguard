import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import React from 'react';
import { useForm } from 'react-hook-form';
import { SubmitHandler } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router';
import { useLocation } from 'react-router-dom';
import { toast } from 'react-toastify';
import * as yup from 'yup';

import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import ToastContent, {
  ToastType,
} from '../../../shared/components/Toasts/ToastContent';
import { isUserAdmin } from '../../../shared/helpers/isUserAdmin';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { patternNoSpecialChars } from '../../../shared/patterns';
import { LoginData } from '../../../shared/types';

type Inputs = {
  username: string;
  password: string;
};

const Login: React.FC = () => {
  const { t } = useTranslation('en');
  const schema = yup
    .object({
      username: yup
        .string()
        .required(t('auth.login.form.required.username'))
        .matches(patternNoSpecialChars, t('form.errors.noSpecialChars')),
      password: yup
        .string()
        .required(t('auth.login.form.required.password'))
        .max(32, t('form.errors.maximumLength', { length: 32 })),
    })
    .required();
  const {
    auth: { login },
    user: { getMe },
  } = useApi();
  const logIn = useAuthStore((state) => state.logIn);
  const navigate = useNavigate();

  const responseErrorToast = (message: string) =>
    toast(<ToastContent message={message} type={ToastType.ERROR} />, {
      toastId: 'login-form-error',
      hideProgressBar: true,
    });

  const { handleSubmit, control, setError } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      password: '',
      username: '',
    },
  });
  const location = useLocation();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const state = location.state as any;

  const loginMutation = useMutation((data: LoginData) => login(data), {
    mutationKey: [MutationKeys.LOG_IN],
    onSuccess: () => {
      getMe().then((user) => {
        logIn(user);
        if (isUserAdmin(user)) {
          navigate(state ? state.path : '/admin/overview', { replace: true });
        } else {
          navigate(state ? state.path : '/me', { replace: true });
        }
      });
    },
    onError: (error: AxiosError) => {
      if (error.response && error.response.status === 401) {
        responseErrorToast('Login failed');
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

  return (
    <section id="login-container">
      <h1>Enter your credentials</h1>
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormInput
          controller={{ control, name: 'username' }}
          placeholder={t('auth.login.form.placeholder.username')}
          innerLabel
          required
        />
        <FormInput
          controller={{ control, name: 'password' }}
          placeholder={t('auth.login.form.placeholder.password')}
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
          text={t('auth.login.form.template.login')}
        />
      </form>
      {/* <p>or</p>
      <Button
        className="big link"
        onClick={() =>
          navigate('../register', {
            replace: false,
          })
        }
      >
        <span>{t('auth.login.form.template.register')}</span>
      </Button> */}
    </section>
  );
};

export default Login;
