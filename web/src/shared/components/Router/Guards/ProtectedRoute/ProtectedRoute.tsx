import { isUndefined } from 'lodash-es';
import React, { ReactNode, useEffect } from 'react';
import { useIdleTimer } from 'react-idle-timer';
import { Navigate, useNavigate } from 'react-router-dom';

import { useAppStore } from '../../../../hooks/store/useAppStore';
import { useAuthStore } from '../../../../hooks/store/useAuthStore';
import useApi from '../../../../hooks/useApi';
import { Settings } from '../../../../types';

interface Props {
  children?: ReactNode;
  allowedGroups?: string[];
  setting?: Setting;
}
type Setting = keyof Settings;
/**
 * Wrapper around Route, check if user is logged in.
 */

const ProtectedRoute: React.FC<Props> = ({
  children,
  allowedGroups,
  setting,
}) => {
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

  if (isUndefined(currentUser)) {
    return <Navigate replace to="/auth/login" />;
  }
  if (settings !== undefined && setting !== undefined) {
    if (!settings[setting]) {
      navigate('/', { replace: true });
    }
  }
  return <>{children}</>;
};
export default ProtectedRoute;
