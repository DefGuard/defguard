import 'tippy.js/dist/svg-arrow.css';
import 'tippy.js/dist/tippy.css';
import 'tippy.js/animations/scale.css';
import './App.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import {
  BrowserRouter as Router,
  Navigate,
  Route,
  Routes,
} from 'react-router-dom';
import shallow from 'zustand/shallow';

import AuthPage from '../../pages/auth/AuthPage';
import LoaderPage from '../../pages/loader/LoaderPage';
import { NetworkPage } from '../../pages/network/NetworkPage';
import OpenidAllowPage from '../../pages/openid/OpenidAllowPage';
import OpenidPage from '../../pages/openid/OpenidPage';
import { OverviewPage } from '../../pages/overview/OverviewPage';
import ProvisionersPage from '../../pages/provisioners/ProvisionersPage';
import { SettingsPage } from '../../pages/settings/SettingsPage';
import { UserProfilePage } from '../../pages/users/UserProfilePage';
import UsersPage from '../../pages/users/UsersPage';
import WebhooksPage from '../../pages/webhooks/WebhooksPage';
import { ToastManager } from '../../shared/components/layout/ToastManager/ToastManager';
import ProtectedRoute from '../../shared/components/Router/Guards/ProtectedRoute/ProtectedRoute';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';
import { useToaster } from '../../shared/hooks/useToaster';
import { QueryKeys } from '../../shared/queries';

const App = () => {
  const toaster = useToaster();
  const {
    getVersion,
    user: { getMe },
    settings: { getSettings },
    license: { getLicense },
  } = useApi();
  const [currentUser, logOut, logIn, isAdmin] = useAuthStore(
    (state) => [state.user, state.logOut, state.logIn, state.isAdmin],
    shallow
  );

  const { isLoading: currentUserLoading, data: userMe } = useQuery(
    [QueryKeys.FETCH_ME],
    getMe,
    {
      onSuccess: (user) => {
        logIn(user);
      },
      onError: () => {
        if (currentUser) {
          logOut();
        }
        console.clear();
      },
      refetchOnMount: true,
      refetchOnWindowFocus: false,
      retry: false,
    }
  );

  const setAppStore = useAppStore((state) => state.setAppStore);

  useQuery([QueryKeys.FETCH_APP_VERSION], getVersion, {
    onSuccess: (data) => {
      setAppStore({ version: data.version });
    },
    onError: (err) => {
      toaster.error('Failed to get application version.');
      console.error(err);
    },
    refetchOnWindowFocus: false,
    retry: false,
  });

  useQuery([QueryKeys.FETCH_SETTINGS], getSettings, {
    onSuccess: (settings) => {
      setAppStore({ settings });
    },
    onError: () => {
      console.clear();
    },
    refetchOnWindowFocus: false,
  });

  useQuery([QueryKeys.FETCH_LICENSE], getLicense, {
    onSuccess: (data) => {
      setAppStore({ license: data });
    },
    onError: () => {
      toaster.error('Failed to fetch licence');
    },
    refetchOnWindowFocus: false,
    enabled: !isUndefined(userMe),
  });

  if (currentUserLoading && !userMe && !currentUser) return <LoaderPage />;

  return (
    <>
      <div id="app">
        <Router>
          <Routes>
            <Route path="auth/*" element={<AuthPage />} />
            <Route path="admin/*">
              <Route index element={<Navigate to="users" />} />
              <Route
                path="network/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <NetworkPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="overview/*"
                element={
                  <ProtectedRoute
                    allowedGroups={['admin']}
                    moduleRequired="wireguard_enabled"
                  >
                    <OverviewPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="users/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <UsersPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="provisioners/*"
                element={
                  <ProtectedRoute
                    allowedGroups={['admin']}
                    moduleRequired="worker_enabled"
                  >
                    <ProvisionersPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="webhooks/*"
                element={
                  <ProtectedRoute
                    allowedGroups={['admin']}
                    moduleRequired="webhooks_enabled"
                  >
                    <WebhooksPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="openid/*"
                element={
                  <ProtectedRoute
                    allowedGroups={['admin']}
                    moduleRequired="openid_enabled"
                  >
                    <OpenidPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="settings/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <SettingsPage />
                  </ProtectedRoute>
                }
              />
              <Route path="*" element={<Navigate to="users" />} />
            </Route>
            <Route
              path="me/*"
              element={
                <ProtectedRoute>
                  <UserProfilePage />
                </ProtectedRoute>
              }
            />
            <Route
              path="consent/*"
              element={
                <ProtectedRoute>
                  <OpenidAllowPage />
                </ProtectedRoute>
              }
            />
            <Route
              path="*"
              element={
                currentUser && isAdmin ? (
                  <Navigate replace to="/admin/overview" />
                ) : (
                  <Navigate replace to="/me" />
                )
              }
            />
          </Routes>
        </Router>
      </div>
      <ToastManager />
    </>
  );
};

export default App;
