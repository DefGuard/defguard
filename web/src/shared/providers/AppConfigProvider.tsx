import { useQuery } from '@tanstack/react-query';
import { type PropsWithChildren, useEffect } from 'react';
import api from '../api/api';
import { isPresent } from '../defguard-ui/utils/isPresent';
import { useApp } from '../hooks/useApp';
import { useAuth } from '../hooks/useAuth';

// useApp queries
// todo: maybe we should wrap it around _authenticated route and make it suspend queries to ensure it's present before page loads ?
export const AppConfigProvider = ({ children }: PropsWithChildren) => {
  const isAuthenticated = useAuth((s) => isPresent(s.user));
  const { data: appInfoResponse } = useQuery({
    queryFn: api.app.info,
    queryKey: ['info'],
    enabled: isAuthenticated,
    refetchOnWindowFocus: true,
    refetchOnReconnect: true,
    refetchOnMount: true,
  });

  useEffect(() => {
    if (isPresent(appInfoResponse)) {
      useApp.setState({
        appInfo: appInfoResponse.data,
      });
    }
  }, [appInfoResponse]);

  return <>{children}</>;
};
