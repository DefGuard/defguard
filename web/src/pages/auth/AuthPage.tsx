import './style.scss';

import { isUndefined } from 'lodash-es';
import { useEffect } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import { useNavigationStore } from '../../shared/hooks/store/useNavigationStore';
import useApi from '../../shared/hooks/useApi';
import { useToaster } from '../../shared/hooks/useToaster';
import { UserMFAMethod } from '../../shared/types';
import { Login } from './Login/Login';
import { MFARoute } from './MFARoute/MFARoute';
import { useMFAStore } from './shared/hooks/useMFAStore';

export const AuthPage = () => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const {
    network: { getNetworks },
  } = useApi();

  const loginSubject = useAuthStore((state) => state.loginSubject);

  const setNavigation = useNavigationStore((state) => state.setState);

  const setAuthStore = useAuthStore((state) => state.setState);

  const [openIdParams, user] = useAuthStore(
    (state) => [state.openIdParams, state.user],
    shallow
  );

  const mfaMethod = useMFAStore((state) => state.mfa_method);

  const [setMFAStore, resetMFAStore] = useMFAStore(
    (state) => [state.setState, state.resetState],
    shallow
  );

  const settings = useAppStore((state) => state.settings);

  const toaster = useToaster();

  useEffect(() => {
    if (user && (!mfaMethod || mfaMethod === UserMFAMethod.NONE) && !openIdParams) {
      navigate('/', { replace: true });
    }
  }, [mfaMethod, navigate, openIdParams, user]);

  useEffect(() => {
    const sub = loginSubject.subscribe(async ({ user, url, mfa }) => {
      // handle openid scenarios first

      // user authenticated but app needs consent
      if (openIdParams && user && !mfa) {
        navigate(`/consent?${openIdParams.toString()}`, { replace: true });
        return;
      }

      // application already had consent from user
      if (url && url.length && user) {
        resetMFAStore();
        window.location.replace(url);
        return;
      }

      // normal auth flow
      if (user) {
        const isAdmin = !isUndefined(user.groups.find((g) => g === 'admin'));
        if (isAdmin) {
          // check if VPN needs wizard
          const networks = await getNetworks();
          if (networks.length === 0) {
            setNavigation({ enableWizard: true });
          } else {
            setNavigation({ enableWizard: false });
          }
        }
        setAuthStore({ user, isAdmin });
        if (!mfa) {
          resetMFAStore();
          if (isAdmin) {
            navigate('/admin/overview', { replace: true });
          } else {
            navigate('/me', { replace: true });
          }
        } else {
          setMFAStore(mfa);
          switch (mfa.mfa_method) {
            case UserMFAMethod.WEB3:
              navigate('../mfa/web3');
              break;
            case UserMFAMethod.WEB_AUTH_N:
              navigate('../mfa/webauthn');
              break;
            case UserMFAMethod.ONE_TIME_PASSWORD:
              navigate('../mfa/totp');
              break;
            default:
              toaster.error(LL.messages.error());
              console.error('API did not return any MFA method in MFA flow.');
              break;
          }
        }
      }
    });
    return () => sub?.unsubscribe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loginSubject, openIdParams]);

  return (
    <div id="auth-container">
      <div className="logo-container">
        {settings ? (
          <img src={settings?.main_logo_url} alt="login_logo" />
        ) : (
          <SvgDefguardLogoLogin />
        )}
      </div>
      <Routes>
        <Route index element={<Navigate to="login" />} />
        <Route path="/" element={<Navigate to="login" />} />
        <Route path="login" element={<Login />} />
        <Route path="mfa/*" element={<MFARoute />} />
        <Route path="*" element={<Navigate to="login" />} />
      </Routes>
    </div>
  );
};
