import { ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';

import { useAppStore } from '../../../../hooks/store/useAppStore';
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
  const openIDRedirect = useAppStore((state) => state.openIDRedirect);

  const navigate = useNavigate();
  if (moduleRequired !== undefined) {
    navigate('/', { replace: true });
  }
  if (openIDRedirect !== undefined || openIDRedirect === false) {
    navigate('/', { replace: true });
  }
  return <>{children}</>;
};
export default openIDRoute;
