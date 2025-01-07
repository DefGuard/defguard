import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';

import { useI18nContext } from '../../../i18n/i18n-react';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { CallbackData } from '../../../shared/types';

type ErrorResponse = {
  msg: string;
};

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

  const callbackMutation = useMutation({
    mutationFn: callback,
    mutationKey: [MutationKeys.OPENID_CALLBACK],
    onSuccess: (data) => loginSubject.next(data),
    onError: (error: AxiosError) => {
      toaster.error(LL.messages.error());
      console.error(error);
      const errorResponse = error.response?.data as ErrorResponse;
      if (errorResponse.msg) {
        setError(errorResponse.msg);
      } else {
        setError(JSON.stringify(error));
      }
    },
    retry: false,
  });

  useEffect(() => {
    if (window.location.search && window.location.search.length > 0) {
      // const hashFragment = window.location.search.substring(1);
      const params = new URLSearchParams(window.location.search);

      // check if error occurred
      const error = params.get('error');

      if (error) {
        setError(error);
        toaster.error(LL.messages.error());
        return;
      }

      const code = params.get('code');
      const state = params.get('state');

      if (code && state) {
        const data: CallbackData = {
          code,
          state,
        };
        callbackMutation.mutate(data);
      } else {
        setError('Expected data not returned by the OpenID provider');
        toaster.error(LL.messages.error());
        return;
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // FIXME: make it a bit more user friendly
  return error ? (
    <div className="error-info">
      <h3>{LL.loginPage.callback.error()}:</h3>
      <p>{error}</p>
      <Button
        id="back-to-login"
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
