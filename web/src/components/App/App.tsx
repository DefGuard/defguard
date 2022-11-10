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
import { UserProfilePage } from '../../pages/users/UserProfilePage';
import UsersPage from '../../pages/users/UsersPage';
import WizardPage from '../../pages/vpn/Wizard/WizardPage';
import WebhooksPage from '../../pages/webhooks/WebhooksPage';
import ProtectedRoute from '../../shared/components/Router/Guards/ProtectedRoute/ProtectedRoute';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';

const App = () => {
  const {
    user: { getMe },
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
                    <UsersPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="provisioners/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <ProvisionersPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="webhooks/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <WebhooksPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="openid/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <OpenidPage />
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
    </>
  );
};

export default App;
