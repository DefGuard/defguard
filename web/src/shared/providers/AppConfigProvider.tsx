import { useQuery } from '@tanstack/react-query';
import { type PropsWithChildren, useEffect } from 'react';
import api from '../api/api';
import { isPresent } from '../defguard-ui/utils/isPresent';
import { useApp } from '../hooks/useApp';
import { useAuth } from '../hooks/useAuth';

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

  const { data: settingsEssentials } = useQuery({
    queryFn: api.settings.getSettingsEssentials,
    queryKey: ['settings-essentials'],
    enabled: isAuthenticated,
    refetchOnWindowFocus: true,
    refetchOnReconnect: true,
    refetchOnMount: true,
  });

  useEffect(() => {
    if (isPresent(settingsEssentials)) {
      useApp.setState({
        settingsEssentials: settingsEssentials.data,
      });
    }
  }, [settingsEssentials]);

  return <>{children}</>;
};
