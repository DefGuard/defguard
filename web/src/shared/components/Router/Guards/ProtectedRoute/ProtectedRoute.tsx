import { isUndefined } from 'lodash-es';
import { ReactNode } from 'react';
import { Navigate } from 'react-router-dom';

import { useAppStore } from '../../../../hooks/store/useAppStore';
import { useAuthStore } from '../../../../hooks/store/useAuthStore';
import { SettingsModules } from '../../../../types';

interface Props {
  children?: ReactNode;
  allowedGroups?: string[];
  moduleRequired?: Setting;
  allowUnauthorized?: boolean;
}

type Setting = keyof SettingsModules;

export const ProtectedRoute = ({
  children,
  allowedGroups,
  moduleRequired,
  allowUnauthorized = false,
}: Props) => {
  const currentUser = useAuthStore((state) => state.user);
  const settings = useAppStore((state) => state.settings);

  // authorized
  if (isUndefined(currentUser) && !allowUnauthorized) {
    console.warn('[GUARD] Not authorized to navigate.');
    return <Navigate replace to="/auth/login" />;
  }

  // have group
  if (allowedGroups && allowedGroups.length > 0 && currentUser) {
    let allowed = false;
    for (const userGroup of currentUser.groups) {
      if (allowedGroups.includes(userGroup)) {
        allowed = true;
      }
    }

    if (!allowed) {
      if (currentUser.groups.includes('admin')) {
        return <Navigate to="/admin/users" replace />;
      } else {
        return <Navigate to="/me" replace />;
      }
    }
  }

  if (isUndefined(settings) && moduleRequired) {
    if (currentUser?.groups.includes('admin')) {
      return <Navigate to="/admin/users" replace />;
    }
    return <Navigate to="/me" replace />;
  }

  // route module is enabled
  if (settings !== undefined && moduleRequired !== undefined) {
    if (!settings[moduleRequired]) {
      console.warn('[GUARD] Not authorized to navigate.');
      if (currentUser?.groups.includes('admin')) {
        return <Navigate to="/admin/users" replace />;
      }
      return <Navigate to="/me" replace />;
    }
  }

  return <>{children}</>;
};
