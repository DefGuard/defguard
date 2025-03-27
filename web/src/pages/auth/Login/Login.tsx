import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery } from '@tanstack/react-query';
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
import { LoaderSpinner } from '../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { patternLoginCharacters } from '../../../shared/patterns';
import { QueryKeys } from '../../../shared/queries';
import { trimObjectStrings } from '../../../shared/utils/trimObjectStrings';
import { OpenIdLoginButton } from './components/OidcButtons';

type Inputs = {
  username: string;
  password: string;
};

export const Login = () => {
  const { LL } = useI18nContext();
  const {
    auth: {
      login,
      openid: { getOpenIdInfo: getOpenidInfo },
    },
  } = useApi();
  const toaster = useToaster();

  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
  const { data: openIdInfo, isLoading: openIdLoading } = useQuery({
    enabled: enterpriseEnabled,
    queryKey: [QueryKeys.FETCH_OPENID_INFO],
    queryFn: getOpenidInfo,
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    retry: false,
  });

  const zodSchema = useMemo(
    () =>
      z.object({
        username: z
          .string()
          .min(1, LL.form.error.minimumLength())
          .max(64)
          .regex(patternLoginCharacters, LL.form.error.forbiddenCharacter()),
        password: z
          .string()
          .min(1, LL.form.error.required())
          .max(128, LL.form.error.maximumLength()),
      }),
    [LL.form.error],
  );

  const { handleSubmit, control, setError } = useForm<Inputs>({
    resolver: zodResolver(zodSchema),
    mode: 'all',
    defaultValues: {
      password: '',
      username: '',
    },
  });

  const loginSubject = useAuthStore((state) => state.loginSubject);

  const loginMutation = useMutation({
    mutationFn: login,
    mutationKey: [MutationKeys.LOG_IN],
    onSuccess: (data) => loginSubject.next(data),
    onError: (error: AxiosError) => {
      if (error.response) {
        switch (error.response.status) {
          case 401: {
            setError(
              'password',
              {
                message: 'username or password is incorrect',
              },
              { shouldFocus: true },
            );
            break;
          }
          case 429: {
            toaster.error(LL.form.error.tooManyBadLoginAttempts());
            break;
          }
          default: {
            console.error(error);
            toaster.error(LL.messages.error());
          }
        }
      } else {
        console.error(error);
        toaster.error(LL.messages.error());
      }
    },
  });

  const onSubmit: SubmitHandler<Inputs> = (data) => {
    if (!loginMutation.isPending) {
      loginMutation.mutate(trimObjectStrings(data));
    }
  };

  return (
    <section id="login-container">
      {!enterpriseEnabled || !openIdLoading ? (
        <>
          <h1>{LL.loginPage.pageTitle()}</h1>
          <form onSubmit={handleSubmit(onSubmit)}>
            <FormInput
              controller={{ control, name: 'username' }}
              placeholder={LL.form.placeholders.username_or_email()}
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
              loading={loginMutation.isPending}
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.PRIMARY}
              text={LL.form.login()}
              data-testid="login-form-submit"
            />
            {openIdInfo && (
              <OpenIdLoginButton
                url={openIdInfo.url}
                display_name={openIdInfo?.button_display_name}
              />
            )}
          </form>
        </>
      ) : (
        <LoaderSpinner size={80} />
      )}
    </section>
  );
};
