import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { lazy, Suspense, useEffect, useRef, useState } from 'react';
// eslint-disable-next-line import/no-unresolved
import { navigatorDetector } from 'typesafe-i18n/detectors';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../i18n/i18n-react';
import { baseLocale, detectLocale, locales } from '../i18n/i18n-util';
import { loadLocaleAsync } from '../i18n/i18n-util.async';
import { LoaderPage } from '../pages/loader/LoaderPage';
import { isUserAdmin } from '../shared/helpers/isUserAdmin';
import { useAppStore } from '../shared/hooks/store/useAppStore';
import { useAuthStore } from '../shared/hooks/store/useAuthStore';
import { useUpdatesStore } from '../shared/hooks/store/useUpdatesStore';
import useApi from '../shared/hooks/useApi';
import { useToaster } from '../shared/hooks/useToaster';
import { QueryKeys } from '../shared/queries';

/**
 * Fetches data needed by app before it's rendered.
 * **/
export const AppLoader = () => {
  const toaster = useToaster();
  const [currentUser, resetAuthState, setAuthState] = useAuthStore(
    (state) => [state.user, state.resetState, state.setState],
    shallow,
  );
  const appSettings = useAppStore((state) => state.settings);
  const {
    getAppInfo,
    user: { getMe },
    getEnterpriseStatus,
    settings: { getEssentialSettings, getEnterpriseSettings },
  } = useApi();
  const [userLoading, setUserLoading] = useState(true);
  const { setLocale } = useI18nContext();
  const activeLanguage = useAppStore((state) => state.language);
  const setAppStore = useAppStore((state) => state.setState);
  const { LL } = useI18nContext();
  const updateEffectRef = useRef(false);
  const setUpdateStore = useUpdatesStore((s) => s.setUpdate);

  useQuery([QueryKeys.FETCH_ME], getMe, {
    onSuccess: async (user) => {
      const isAdmin = isUserAdmin(user);
      setAuthState({ isAdmin, user });
      setUserLoading(false);
    },
    onError: () => {
      if (currentUser) {
        resetAuthState();
      }
      setUserLoading(false);
    },
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    retry: false,
  });

  useQuery([QueryKeys.FETCH_APP_INFO], getAppInfo, {
    onSuccess: (data) => {
      setAppStore({ appInfo: data });
    },
    onError: (err) => {
      toaster.error(LL.messages.errorVersion());
      console.error(err);
    },
    refetchOnWindowFocus: false,
    retry: false,
    enabled: !isUndefined(currentUser),
  });

  useQuery([QueryKeys.FETCH_ENTERPRISE_SETTINGS], getEnterpriseSettings, {
    onSuccess: (settings) => {
      setAppStore({ enterprise_settings: settings });
    },
    onError: (err) => {
      console.error(err);
    },
    refetchOnWindowFocus: true,
    retry: false,
    enabled: !isUndefined(currentUser),
  });

  useQuery([QueryKeys.FETCH_ENTERPRISE_STATUS], getEnterpriseStatus, {
    onSuccess: (status) => {
      setAppStore({
        enterprise_status: status,
      });
    },
    onError: (err) => {
      // FIXME: Add a proper error message
      toaster.error(LL.messages.errorVersion());
      console.error(err);
    },
    refetchOnWindowFocus: false,
    retry: false,
  });

  const { isLoading: settingsLoading, data: essentialSettings } = useQuery(
    [QueryKeys.FETCH_ESSENTIAL_SETTINGS],
    getEssentialSettings,
    {
      refetchOnWindowFocus: true,
      refetchOnMount: true,
    },
  );

  useEffect(() => {
    if (!activeLanguage) {
      let lang = detectLocale(navigatorDetector);
      if (!locales.includes(lang)) {
        lang = baseLocale;
      }
      setAppStore({ language: lang });
    } else {
      if (locales.includes(activeLanguage)) {
        loadLocaleAsync(activeLanguage).then(() => {
          setLocale(activeLanguage);
          document.documentElement.setAttribute('lang', activeLanguage);
        });
      } else {
        setAppStore({ language: baseLocale });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeLanguage]);

  // setAppSettings
  useEffect(() => {
    if (essentialSettings) {
      if (document.title !== essentialSettings.instance_name) {
        document.title = essentialSettings.instance_name;
      }
      setAppStore({ settings: essentialSettings });
    }
  }, [essentialSettings, setAppStore]);

  //TODO: Check for updates from core here :3
  useEffect(() => {
    if (!updateEffectRef.current) {
      setUpdateStore({
        critical: true,
        notes: `
# Quick fix release

- Allow usernames with minimum 1 character by @moubctez in #878
- Okta fixes - more about Okta integration in our docs - fallback to calling user-info if claims not present in the ID token by @t-aleksander in #883
        `,
        releaseLink: 'https://github.com/DefGuard/defguard/releases/tag/v1.1.3',
        version: '1.1.3',
      });
      updateEffectRef.current = true;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (userLoading || (settingsLoading && isUndefined(appSettings))) {
    return <LoaderPage />;
  }

  return (
    <Suspense fallback={<LoaderPage />}>
      <App />
    </Suspense>
  );
};

const App = lazy(() => import('./App/App'));
