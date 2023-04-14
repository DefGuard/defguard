import { isUndefined } from 'lodash-es';
import { ReactNode, useEffect } from 'react';
import { useIdleTimer } from 'react-idle-timer';
import { Navigate, useNavigate } from 'react-router-dom';

import { useAppStore } from '../../../../hooks/store/useAppStore';
import { useAuthStore } from '../../../../hooks/store/useAuthStore';
import useApi from '../../../../hooks/useApi';
import { Settings } from '../../../../types';

interface Props {
  children?: ReactNode;
  allowedGroups?: string[];
  moduleRequired?: Setting;
  allowUnauthorized?: boolean;
}
type Setting = keyof Settings;

export const ProtectedRoute = ({
  children,
  allowedGroups,
  moduleRequired,
  allowUnauthorized = false,
}: Props) => {
  const currentUser = useAuthStore((state) => state.user);
  const settings = useAppStore((state) => state.settings);
  const {
    auth: { logout },
  } = useApi();

  const handleOnIdle = () => {
    logout();
  };
  const navigate = useNavigate();
  useIdleTimer({
    timeout: 10 * 60 * 10000,
    onIdle: handleOnIdle,
    debounce: 500,
  });

  useEffect(() => {
    if (currentUser && allowedGroups && allowedGroups.length > 0) {
      let allowed = false;
      for (const group of currentUser.groups) {
        if (allowedGroups.includes(group)) {
          allowed = true;
          break;
        }
      }
      if (!allowed) {
        console.warn('[GUARD] Not authorized to navigate.');
        navigate('/', { replace: true });
      }
    }
  }, [allowedGroups, currentUser, navigate]);

  if (isUndefined(currentUser) && !allowUnauthorized) {
    console.warn('[GUARD] Not authorized to navigate.');
    return <Navigate replace to="/auth/login" />;
  }

  if (settings !== undefined && moduleRequired !== undefined) {
    if (!settings[moduleRequired]) {
      console.warn('[GUARD] Not authorized to navigate.');
      navigate('/', { replace: true });
    }
  }
  return <>{children}</>;
};
