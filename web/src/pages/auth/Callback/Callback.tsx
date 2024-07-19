import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';

import { useEffect, useState } from 'react';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { CallbackData } from '../../../shared/types';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { useToaster } from '../../../shared/hooks/useToaster';
import { useI18nContext } from '../../../i18n/i18n-react';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import { useNavigate } from 'react-router';

export const OpenIDCallback = () => {
  const {
    auth: {
      openid: { callback },
    },
  } = useApi();
  const loginSubject = useAuthStore((state) => state.loginSubject);
  const toaster = useToaster();
  const { LL } = useI18nContext();
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  const callbackMutation = useMutation((data: CallbackData) => callback(data), {
    mutationKey: [MutationKeys.OPENID_CALLBACK],
    onSuccess: (data) => loginSubject.next(data),
    onError: (error: AxiosError) => {
      toaster.error(LL.messages.error());
      console.error(error);
    },
    retry: false,
  });

  useEffect(() => {
    if (window.location.hash && window.location.hash.length > 0) {
      const hashFragment = window.location.hash.substring(1);
      const params = new URLSearchParams(hashFragment);

      // check if error occured
      const error = params.get('error');

      if (error) {
        setError(error);
        toaster.error(LL.messages.error());
        return;
      }

      const id_token = params.get('id_token');
      const state = params.get('state');

      if (id_token && state) {
        const data: CallbackData = {
          id_token,
          state,
        };
        callbackMutation.mutate(data);
      }
    }
  }, []);

  // TODO: Perhaphs make it a bit more user friendly
  return error ? (
    <div className="error-info">
      <p>
        {LL.loginPage.callback.error()}: {error}
      </p>
      <Button
        text={LL.loginPage.callback.return()}
        onClick={() => {
          navigate('/auth/login');
        }}
      />
    </div>
  ) : (
    <LoaderSpinner size={80} />
  );
};
