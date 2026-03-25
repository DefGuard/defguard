import { useQuery } from '@tanstack/react-query';
import { type PropsWithChildren, useEffect, useRef } from 'react';
import api from '../api/api';
import { isPresent } from '../defguard-ui/utils/isPresent';
import { openModal } from '../hooks/modalControls/modalsSubjects';
import { ModalName } from '../hooks/modalControls/modalTypes';
import { useApp } from '../hooks/useApp';
import { useAuth } from '../hooks/useAuth';

const DISMISSED_UPDATE_KEY = 'dismissed-update-version';

export const AppInfoProvider = ({ children }: PropsWithChildren) => {
  const isAuthenticated = useAuth((s) => isPresent(s.user));
  const updateModalOpenedRef = useRef(false);

  const { data: appInfo } = useQuery({
    queryFn: api.app.info,
    queryKey: ['info'],
    enabled: isAuthenticated,
    refetchOnWindowFocus: true,
    refetchOnReconnect: true,
    refetchOnMount: true,
    select: (resp) => resp.data,
  });

  const { data: update } = useQuery({
    queryFn: api.app.updates,
    queryKey: ['app-updates'],
    enabled: isAuthenticated,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchOnMount: true,
    select: (resp) => resp.data,
  });

  useEffect(() => {
    if (appInfo) {
      useApp.setState({ appInfo });
    }
  }, [appInfo]);

  useEffect(() => {
    if (!update || updateModalOpenedRef.current) return;

    const dismissedVersion = localStorage.getItem(DISMISSED_UPDATE_KEY);
    if (dismissedVersion === update.version) return;

    updateModalOpenedRef.current = true;
    openModal(ModalName.AppUpdate, update);
  }, [update]);

  return <>{children}</>;
};
