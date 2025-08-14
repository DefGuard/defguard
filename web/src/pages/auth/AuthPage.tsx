import './style.scss';

import { useEffect, useMemo, useState } from 'react';
import { Navigate, Route, Routes, useNavigate, useSearchParams } from 'react-router-dom';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
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

const VALID_URL_PATTERN =
  /^(https?:\/\/[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9](?:\.[a-zA-Z]{2,})+(?::[0-9]{1,5})?(?:\/[a-zA-Z0-9\-._~%!$&'()*+,;=:@/]*)?(?:\?[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?]*)?|\/[a-zA-Z0-9\-._~%!$&'()*+,;=:@/]*(?:\?[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?]*)?)$/gi;

// Return redirect URL only if it matches a safe pattern:
// - starts with http/https
// - contains only safe characters (no <, >)
// - can include query params
//
// Once a URL matches this pattern we also explicitly check for unsafe elements in case they are a part of redirect URL query params
const sanitizeRedirectUrl = (url: string | null) => {
  if (url?.match(VALID_URL_PATTERN) && !/javascript:|data:|\\/.test(url)) return url;

  return null;
};

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

  const [params] = useSearchParams();
  const redirectUrl = useMemo(() => sanitizeRedirectUrl(params.get('r')), [params]);

  useEffect(() => {
    if (user && (!mfaMethod || mfaMethod === UserMFAMethod.NONE) && !openIdParams) {
      navigate('/', { replace: true });
    }
  }, [mfaMethod, navigate, openIdParams, user]);

  useEffect(() => {
    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    const sub = loginSubject.subscribe(async ({ user, url, mfa }): Promise<void> => {
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
        let navigateURL = '/me';
        if (user.is_admin) {
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
        setAuthStore({ user });
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
        <a target="_blank" href="https://defguard.net" rel="noreferrer noopener">
          {settings ? (
            <img src={settings?.main_logo_url} alt="login_logo" />
          ) : (
            <SvgDefguardLogoLogin />
          )}
        </a>
      </div>
      <Routes>
        <Route index element={<Navigate to="login" />} />
        <Route path="/" element={<Navigate to="login" />} />
        <Route path="login" element={<Login />} />
        <Route path="mfa/*" element={<MFARoute />} />
        <Route path="callback" element={<OpenIDCallback />} />
        <Route path="*" element={<Navigate to="login" />} />
      </Routes>
    </div>
  );
};
