import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { lazy, Suspense, useEffect, useState } from 'react';
// eslint-disable-next-line import/no-unresolved
import { navigatorDetector } from 'typesafe-i18n/detectors';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../i18n/i18n-react';
import { detectLocale } from '../i18n/i18n-util';
import { loadLocaleAsync } from '../i18n/i18n-util.async';
import LoaderPage from '../pages/loader/LoaderPage';
import { isUserAdmin } from '../shared/helpers/isUserAdmin';
import { useAppStore } from '../shared/hooks/store/useAppStore';
import { useAuthStore } from '../shared/hooks/store/useAuthStore';
import { useNavigationStore } from '../shared/hooks/store/useNavigationStore';
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
    shallow
  );
  const appSettings = useAppStore((state) => state.settings);
  const {
    getVersion,
    user: { getMe },
    settings: { getSettings },
    license: { getLicense },
    network: { getNetworks },
  } = useApi();
  const [userLoading, setUserLoading] = useState(true);
  const { setLocale } = useI18nContext();
  const localLanguage = useAppStore((state) => state.language);
  const setAppStore = useAppStore((state) => state.setAppStore);
  const setNavigation = useNavigationStore((state) => state.setState);
  const license = useAppStore((state) => state.license);
  const { LL } = useI18nContext();

  useQuery([QueryKeys.FETCH_ME], getMe, {
    onSuccess: async (user) => {
      const isAdmin = isUserAdmin(user);
      if (isAdmin) {
        const networks = await getNetworks();
        if (networks.length === 0) {
          setNavigation({ enableWizard: true });
        } else {
          setNavigation({ enableWizard: false });
        }
      }
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

  useQuery([QueryKeys.FETCH_APP_VERSION], getVersion, {
    onSuccess: (data) => {
      setAppStore({ version: data.version });
    },
    onError: (err) => {
      toaster.error(LL.messages.errorVersion());
      console.error(err);
    },
    refetchOnWindowFocus: false,
    retry: false,
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
    }
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
    if (!localLanguage) {
      const lang = detectLocale(navigatorDetector);
      setAppStore({ language: lang });
    } else {
      loadLocaleAsync(localLanguage).then(() => {
        setLocale(localLanguage);
        document.documentElement.setAttribute('lang', localLanguage);
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [localLanguage]);

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
  )
    return <LoaderPage />;

  return (
    <Suspense fallback={<LoaderPage />}>
      <App />
    </Suspense>
  );
};

const App = lazy(() => import('./App/App'));
