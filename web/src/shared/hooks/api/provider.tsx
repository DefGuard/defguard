import axios from 'axios';
import { PropsWithChildren, useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { removeNulls } from '../../utils/removeNulls';
import { useApiStore } from './store';

const envBaseUrl: string | undefined = import.meta.env.VITE_API_BASE_URL;

const ApiContextManager = ({ children }: PropsWithChildren) => {
  const [client, endpoints] = useApiStore((s) => [s.client, s.endpoints], shallow);
  const initApiStore = useApiStore((s) => s.init, shallow);

  const { LL } = useI18nContext();

  useEffect(() => {
    if (!isPresent(client)) {
      const res = axios.create({
        baseURL: envBaseUrl && String(envBaseUrl).length > 0 ? envBaseUrl : '/api/v1',
      });

      res.defaults.headers.common['Content-Type'] = 'application/json';
      initApiStore(res);
    }
  }, [client, initApiStore]);

  useEffect(() => {
    if (client) {
      const defaultResponseInterceptor = client.interceptors.response.use(
        (res) => {
          // API sometimes returns null in optional fields.
          if (res.data) {
            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            res.data = removeNulls(res.data);
          }
          return res;
        },
        (error) => {
          console.error('Axios Error ', error);
        },
      );
      return () => {
        client.interceptors.response.eject(defaultResponseInterceptor);
      };
    }
  }, [LL.messages, client]);

  if (!client || !endpoints) return null;

  return <>{children}</>;
};

export const ApiProvider = ({ children }: PropsWithChildren) => {
  return <ApiContextManager>{children}</ApiContextManager>;
};
