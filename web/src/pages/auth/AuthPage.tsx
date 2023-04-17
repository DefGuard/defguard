import './style.scss';

import { useEffect } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router-dom';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import { isUserAdmin } from '../../shared/helpers/isUserAdmin';
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

  const wizardEnabled = useNavigationStore((state) => state.enableWizard);

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

      if (mfa) {
        setMFAStore(mfa);
        let mfaUrl = '';
        switch (mfa.mfa_method) {
          case UserMFAMethod.WEB3:
            mfaUrl = '/auth/mfa/web3';
            break;
          case UserMFAMethod.WEB_AUTH_N:
            mfaUrl = '/auth/mfa/webauthn';
            break;
          case UserMFAMethod.ONE_TIME_PASSWORD:
            mfaUrl = '/auth/mfa/totp';
            break;
          default:
            toaster.error(LL.messages.error());
            console.error('API did not return any MFA method in MFA flow.');
            return;
        }
        navigate(mfaUrl, { replace: true });
        return;
      }

      // authorization finished
      if (user) {
        const isAdmin = isUserAdmin(user);
        let navigateURL = '/me';
        if (isAdmin) {
          // check if VPN needs wizard
          if (!wizardEnabled) {
            const networks = await getNetworks();
            if (networks.length === 0) {
              setNavigation({ enableWizard: true });
              navigateURL = '/admin/wizard';
            } else {
              setNavigation({ enableWizard: false });
              navigateURL = '/admin/overview';
            }
          }
        }
        setAuthStore({ user, isAdmin });
        resetMFAStore();
        navigate(navigateURL, { replace: true });
      }
    });
    return () => sub?.unsubscribe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loginSubject, openIdParams, wizardEnabled]);

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
