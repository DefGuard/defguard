import './style.scss';

import { useEffect, useState } from 'react';
import { Navigate, Route, Routes, useNavigate, useSearchParams } from 'react-router-dom';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import { isUserAdmin } from '../../shared/helpers/isUserAdmin';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';
import { useToaster } from '../../shared/hooks/useToaster';
import { UserMFAMethod } from '../../shared/types';
import { RedirectPage } from '../redirect/RedirectPage';
import { OpenIDCallback } from './Callback/Callback';
import { Login } from './Login/Login';
import { MFARoute } from './MFARoute/MFARoute';
import { useMFAStore } from './shared/hooks/useMFAStore';

export const AuthPage = () => {
  const {
    getAppInfo,
    settings: { getSettings },
  } = useApi();
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const [showRedirect, setShowRedirect] = useState(false);

  const loginSubject = useAuthStore((state) => state.loginSubject);

  const setAuthStore = useAuthStore((state) => state.setState);

  const [openIdParams, user] = useAuthStore(
    (state) => [state.openIdParams, state.user],
    shallow,
  );

  const mfaMethod = useMFAStore((state) => state.mfa_method);

  const [setMFAStore, resetMFAStore] = useMFAStore(
    (state) => [state.setState, state.resetState],
    shallow,
  );

  const settings = useAppStore((state) => state.settings);

  const toaster = useToaster();

  const setAppStore = useAppStore((state) => state.setState);

  const enterpriseEnabled = useAppStore((state) => state.enterprise_status?.enabled);

  const [params] = useSearchParams();
  const redirectUrl = params.get('r');

  useEffect(() => {
    if (user && (!mfaMethod || mfaMethod === UserMFAMethod.NONE) && !openIdParams) {
      navigate('/', { replace: true });
    }
  }, [mfaMethod, navigate, openIdParams, user]);

  useEffect(() => {
    const sub = loginSubject.subscribe(async ({ user, url, mfa }) => {
      // handle forward auth redirect
      if (redirectUrl && user) {
        setShowRedirect(true);
        resetMFAStore();
        window.location.replace(redirectUrl);
        return;
      }

      // handle openid scenarios

      // user authenticated but app needs consent
      if (openIdParams && user && !mfa) {
        navigate(`/consent?${openIdParams.toString()}`, { replace: true });
        return;
      }

      // application already had consent from user
      if (url && url.length && user) {
        setShowRedirect(true);
        resetMFAStore();
        window.location.replace(url);
        return;
      }

      if (mfa) {
        setMFAStore(mfa);
        let mfaUrl = '';
        switch (mfa.mfa_method) {
          case UserMFAMethod.WEB_AUTH_N:
            mfaUrl = '/auth/mfa/webauthn';
            break;
          case UserMFAMethod.ONE_TIME_PASSWORD:
            mfaUrl = '/auth/mfa/totp';
            break;
          case UserMFAMethod.EMAIL:
            mfaUrl = '/auth/mfa/email';
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
          // check where to navigate administrator
          const appInfo = await getAppInfo();
          const settings = await getSettings();
          setAppStore({
            appInfo,
            settings,
          });
          if (settings.wireguard_enabled) {
            if (!appInfo?.network_present) {
              navigateURL = '/admin/wizard';
            } else {
              navigateURL = '/admin/overview';
            }
          } else {
            navigateURL = '/admin/users';
          }
        }
        setAuthStore({ user, isAdmin });
        resetMFAStore();
        navigate(navigateURL, { replace: true });
      }
    });
    return () => sub?.unsubscribe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loginSubject, openIdParams, redirectUrl]);

  if (showRedirect) return <RedirectPage />;

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
        {enterpriseEnabled && <Route path="callback" element={<OpenIDCallback />} />}
        <Route path="*" element={<Navigate to="login" />} />
      </Routes>
    </div>
  );
};
