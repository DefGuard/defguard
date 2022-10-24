import 'react-toastify/dist/ReactToastify.css';
import 'tippy.js/dist/svg-arrow.css';
import 'tippy.js/dist/tippy.css';
import 'tippy.js/animations/scale.css';
import './App.scss';

import React, { Suspense, useEffect, useMemo, useState } from 'react';
import {
  BrowserRouter as Router,
  Navigate,
  Route,
  Routes,
} from 'react-router-dom';
import { ToastContainer, ToastOptions } from 'react-toastify';
import useBreakpoint from 'use-breakpoint';
import shallow from 'zustand/shallow';

import AuthPage from '../../pages/auth/AuthPage';
import LoaderPage from '../../pages/loader/LoaderPage';
import OpenidPage from '../../pages/openid/OpenidPage';
import { OverviewPage } from '../../pages/overview/OverviewPage';
import ProvisionersPage from '../../pages/provisioners/ProvisionersPage';
import { UserProfilePage } from '../../pages/users/UserProfilePage';
import WizardPage from '../../pages/vpn/Wizard/WizardPage';
import WebhooksPage from '../../pages/webhooks/WebhooksPage';
import ProtectedRoute from '../../shared/components/Router/Guards/ProtectedRoute/ProtectedRoute';
import ToastifyCloseButton from '../../shared/components/Toasts/CloseButton';
import {
  standardToastConfig,
  standardToastConfigMobile,
} from '../../shared/components/Toasts/toastConfigs';
import { deviceBreakpoints } from '../../shared/constants';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';

const OAuthPage = React.lazy(() => import('../../pages/oauth/OAuthPage'));
const UsersPage = React.lazy(() => import('../../pages/users/UsersPage'));
const OpenidAllowPage = React.lazy(
  () => import('../../pages/openid/OpenidAllowPage')
);

const App: React.FC = () => {
  const [meCheckLoading, setMeCheckLoading] = useState(true);

  const {
    user: { getMe },
  } = useApi();
  const [currentUser, logOut, logIn, isAdmin] = useAuthStore(
    (state) => [state.user, state.logOut, state.logIn, state.isAdmin],
    shallow
  );
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const getToastDefaultConfig: ToastOptions = useMemo(() => {
    return breakpoint === 'mobile'
      ? standardToastConfigMobile
      : standardToastConfig;
  }, [breakpoint]);

  useEffect(() => {
    getMe()
      .then((user) => {
        logIn(user);
        setMeCheckLoading(false);
      })
      .catch(() => {
        if (currentUser) {
          logOut();
        }
        setMeCheckLoading(false);
        console.clear();
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (meCheckLoading) return <LoaderPage />;

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
                  <ProtectedRoute allowedGroups={['admin']}>
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
                    <Suspense fallback={<LoaderPage />}>
                      <UsersPage />
                    </Suspense>
                  </ProtectedRoute>
                }
              />
              <Route
                path="provisioners/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <Suspense fallback={<LoaderPage />}>
                      <ProvisionersPage />
                    </Suspense>
                  </ProtectedRoute>
                }
              />
              <Route
                path="webhooks/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <Suspense fallback={<LoaderPage />}>
                      <WebhooksPage />
                    </Suspense>
                  </ProtectedRoute>
                }
              />
              <Route
                path="openid/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <Suspense fallback={<LoaderPage />}>
                      <OpenidPage />
                    </Suspense>
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
                <Suspense fallback={<LoaderPage />}>
                  <OAuthPage />
                </Suspense>
              }
            />
            <Route
              path="openid/authorize/*"
              element={
                <Suspense fallback={<LoaderPage />}>
                  <OpenidAllowPage />
                </Suspense>
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
      <ToastContainer
        {...getToastDefaultConfig}
        closeButton={ToastifyCloseButton}
      />
    </>
  );
};

export default App;
