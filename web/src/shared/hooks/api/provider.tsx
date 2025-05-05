import { PropsWithChildren, useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import { removeNulls } from '../../utils/removeNulls';
import { useApiStore } from './store';

const ApiContextManager = ({ children }: PropsWithChildren) => {
  const [client, endpoints] = useApiStore((s) => [s.client, s.endpoints], shallow);

  const { LL } = useI18nContext();

  useEffect(() => {
    if (client && LL && LL.messages) {
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [LL?.messages, client]);

  if (!client || !endpoints) return null;

  return <>{children}</>;
};

export const ApiProvider = ({ children }: PropsWithChildren) => {
  return <ApiContextManager>{children}</ApiContextManager>;
};
