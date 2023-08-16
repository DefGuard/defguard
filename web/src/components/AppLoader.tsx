import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { lazy, Suspense, useEffect, useState } from 'react';
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
    settings: { getSettings },
    license: { getLicense },
  } = useApi();
  const [userLoading, setUserLoading] = useState(true);
  const { setLocale } = useI18nContext();
  const activeLanguage = useAppStore((state) => state.language);
  const setAppStore = useAppStore((state) => state.setAppStore);
  const license = useAppStore((state) => state.license);
  const { LL } = useI18nContext();

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

  const { isLoading: settingsLoading } = useQuery(
    [QueryKeys.FETCH_SETTINGS],
    getSettings,
    {
      onSuccess: (settings) => {
        setAppStore({ settings });
      },
      onError: () => {
        console.clear();
      },
      refetchOnWindowFocus: false,
    },
  );

  const { isLoading: licenseLoading } = useQuery([QueryKeys.FETCH_LICENSE], getLicense, {
    onSuccess: (data) => {
      setAppStore({ license: data });
    },
    onError: () => {
      toaster.error(LL.messages.errorLicense());
    },
    refetchOnWindowFocus: false,
    refetchOnMount: false,
  });

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

  useEffect(() => {
    if (appSettings && appSettings.instance_name) {
      if (document.title !== appSettings.instance_name) {
        document.title = appSettings.instance_name;
      }
    }
  }, [appSettings]);

  if (
    userLoading ||
    (settingsLoading && isUndefined(appSettings)) ||
    (licenseLoading && isUndefined(license))
  ) {
    return <LoaderPage />;
  }

  return (
    <Suspense fallback={<LoaderPage />}>
      <App />
    </Suspense>
  );
};

const App = lazy(() => import('./App/App'));
