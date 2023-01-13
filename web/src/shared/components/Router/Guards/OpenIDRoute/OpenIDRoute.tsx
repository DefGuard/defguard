import { ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';

import { useAppStore } from '../../../../hooks/store/useAppStore';
import { useOpenIDStore } from '../../../../hooks/store/useOpenIdStore';
import { Settings } from '../../../../types';

interface Props {
  children?: ReactNode;
  moduleRequired?: Setting;
}
type Setting = keyof Settings;

/**
 * Wrapper around Route, check if user came from openID redirect
 */

const openIDRoute = ({ children, moduleRequired }: Props) => {
  const openIDRedirect = useOpenIDStore((state) => state.openIDRedirect);
  const settings = useAppStore((state) => state.settings);

  const navigate = useNavigate();
  if (settings !== undefined && moduleRequired !== undefined) {
    if (!settings[moduleRequired]) {
      navigate('/', { replace: true });
    }
  }
  if (openIDRedirect === false) {
    navigate('/', { replace: true });
  }
  return <>{children}</>;
};
export default openIDRoute;
