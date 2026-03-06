import { useQuery } from '@tanstack/react-query';
import { Fragment, type PropsWithChildren, useEffect } from 'react';
import api from '../api/api';
import { useAuth } from '../hooks/useAuth';
import { getSessionInfoQueryOptions } from '../query';

export const AppUserProvider = ({ children }: PropsWithChildren) => {
  const { data: sessionInfo } = useQuery(getSessionInfoQueryOptions);

  const { data: meData } = useQuery({
    queryFn: api.user.getMe,
    queryKey: ['me'],
    refetchOnWindowFocus: true,
    throwOnError: false,
    select: (resp) => resp.data,
    enabled: sessionInfo?.authorized,
  });

  useEffect(() => {
    if (meData) {
      useAuth.getState().setUser(meData);
    }
  }, [meData]);

  return <Fragment>{children}</Fragment>;
};
