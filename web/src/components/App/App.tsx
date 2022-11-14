import 'tippy.js/dist/svg-arrow.css';
import 'tippy.js/dist/tippy.css';
import 'tippy.js/animations/scale.css';
import './App.scss';

import { useQuery } from '@tanstack/react-query';
import {
  BrowserRouter as Router,
  Navigate,
  Route,
  Routes,
} from 'react-router-dom';
import shallow from 'zustand/shallow';

import AuthPage from '../../pages/auth/AuthPage';
import LoaderPage from '../../pages/loader/LoaderPage';
import OAuthPage from '../../pages/oauth/OAuthPage';
import OpenidAllowPage from '../../pages/openid/OpenidAllowPage';
import OpenidPage from '../../pages/openid/OpenidPage';
import { OverviewPage } from '../../pages/overview/OverviewPage';
import ProvisionersPage from '../../pages/provisioners/ProvisionersPage';
import { SettingsPage } from '../../pages/settings/SettingsPage';
import { UserProfilePage } from '../../pages/users/UserProfilePage';
import UsersPage from '../../pages/users/UsersPage';
import WizardPage from '../../pages/vpn/Wizard/WizardPage';
import WebhooksPage from '../../pages/webhooks/WebhooksPage';
import { ToastManager } from '../../shared/components/layout/ToastManager/ToastManager';
import ProtectedRoute from '../../shared/components/Router/Guards/ProtectedRoute/ProtectedRoute';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';

const App = () => {
  const {
    user: { getMe },
    settings: { getSettings },
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
  const [setAppStore] = useAppStore((state) => [state.setAppStore], shallow);

  useQuery([QueryKeys.FETCH_SETTINGS], getSettings, {
    onSuccess: (settings) => {
      setAppStore({ settings });
    },
    onError: () => {
      console.clear();
    },
    enabled: !userMe,
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    retry: true,
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
                path="overview/*"
                element={
                  <ProtectedRoute
                    allowedGroups={['admin']}
                    setting={'wireguard_enabled'}
                  >
                    <OverviewPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="wizard/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <WizardPage />
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
                    setting={'worker_enabled'}
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
                    setting={'webhooks_enabled'}
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
                    setting={'openid_enabled'}
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
            <Route path="consent/*" element={<OAuthPage />} />
            <Route path="openid/authorize/*" element={<OpenidAllowPage />} />
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
