import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { lazy, Suspense, useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../i18n/i18n-react';
import { LoaderPage } from '../pages/loader/LoaderPage';
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
    getNewVersion,
    user: { getMe },
    settings: { getEssentialSettings, getEnterpriseSettings },
  } = useApi();
  const setAppStore = useAppStore((state) => state.setState);
  const { LL } = useI18nContext();
  const setUpdateStore = useUpdatesStore((s) => s.setUpdate);

  const {
    data: meData,
    isLoading: userLoading,
    error: meFetchError,
  } = useQuery({
    queryFn: getMe,
    queryKey: [QueryKeys.FETCH_ME],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    retry: false,
  });

  useEffect(() => {
    if (meFetchError && currentUser) {
      if (currentUser) {
        resetAuthState();
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [meFetchError]);

  useEffect(() => {
    if (meData) {
      setAuthState({ user: meData });
    }
  }, [meData, setAuthState]);

  const { data: appInfoData, error: appInfoError } = useQuery({
    queryFn: getAppInfo,
    queryKey: [QueryKeys.FETCH_APP_INFO],
    refetchOnWindowFocus: true,
    refetchOnMount: true,
    enabled: !isUndefined(currentUser),
  });

  useEffect(() => {
    if (appInfoError) {
      toaster.error(LL.messages.errorVersion());
      console.error(appInfoError);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [appInfoError]);

  useEffect(() => {
    if (appInfoData) {
      setAppStore({ appInfo: appInfoData });
    }
  }, [appInfoData, setAppStore]);

  const { data: enterpriseSettingsData, error: enterpriseSettingsError } = useQuery({
    queryFn: getEnterpriseSettings,
    queryKey: [QueryKeys.FETCH_ENTERPRISE_SETTINGS],
    refetchOnWindowFocus: true,
    retry: false,
    enabled: !isUndefined(currentUser),
  });

  useEffect(() => {
    if (enterpriseSettingsError) {
      console.error(enterpriseSettingsError);
    }
  }, [enterpriseSettingsError]);

  useEffect(() => {
    setAppStore({ enterprise_settings: enterpriseSettingsData });
  }, [setAppStore, enterpriseSettingsData]);

  const { isLoading: settingsLoading, data: essentialSettings } = useQuery({
    queryFn: getEssentialSettings,
    queryKey: [QueryKeys.FETCH_ESSENTIAL_SETTINGS],
    refetchOnMount: true,
  });

  // setAppSettings
  useEffect(() => {
    if (essentialSettings) {
      if (document.title !== essentialSettings.instance_name) {
        document.title = essentialSettings.instance_name;
      }
      setAppStore({ settings: essentialSettings });
    }
  }, [essentialSettings, setAppStore]);

  const { data: newVersionData, error: newVersionError } = useQuery({
    queryFn: getNewVersion,
    queryKey: [QueryKeys.FETCH_NEW_VERSION],
    refetchOnWindowFocus: false,
    refetchOnMount: true,
    enabled: !isUndefined(currentUser) && currentUser.is_admin,
  });

  useEffect(() => {
    if (newVersionError) {
      console.error(newVersionError);
    }
  }, [newVersionError]);

  useEffect(() => {
    if (newVersionData) {
      setUpdateStore(newVersionData);
    }
  }, [newVersionData, setUpdateStore]);

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
