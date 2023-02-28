import 'tippy.js/dist/svg-arrow.css';
import 'tippy.js/dist/tippy.css';
import 'tippy.js/animations/scale.css';
import './App.scss';

import {
  BrowserRouter as Router,
  Navigate,
  Route,
  Routes,
} from 'react-router-dom';

import { OpenidAllowPage } from '../../pages/allow/OpenidAllowPage';
import AuthPage from '../../pages/auth/AuthPage';
import { NetworkPage } from '../../pages/network/NetworkPage';
import { OpenidClientsListPage } from '../../pages/openid/OpenidClientsListPage/OpenidClientsListPage';
import { OverviewPage } from '../../pages/overview/OverviewPage';
import { ProvisionersPage } from '../../pages/provisioners/ProvisionersPage';
import { SettingsPage } from '../../pages/settings/SettingsPage';
import { UserProfile } from '../../pages/users/UserProfile/UserProfile';
import { UsersPage } from '../../pages/users/UsersPage';
import { UsersSharedModals } from '../../pages/users/UsersSharedModals';
import { WebhooksListPage } from '../../pages/webhooks/WebhooksListPage';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { ToastManager } from '../../shared/components/layout/ToastManager/ToastManager';
import ProtectedRoute from '../../shared/components/Router/Guards/ProtectedRoute/ProtectedRoute';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import OpenIDRoute from '../../shared/components/Router/Guards/OpenIDRoute/OpenIDRoute';
import WizardPage from '../../pages/vpn/Wizard/WizardPage';

const App = () => {
  const currentUser = useAuthStore((state) => state.user);
  const isAdmin = useAuthStore((state) => state.isAdmin);
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
                path="wizard/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <WizardPage />
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
                    <WebhooksListPage />
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
                    <OpenidClientsListPage />
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
                  <PageContainer>
                    <UserProfile />
                    <UsersSharedModals />
                  </PageContainer>
                </ProtectedRoute>
              }
            />
            <Route
              path="consent/*"
              element={
                <OpenIDRoute moduleRequired="openid_enabled">
                  <OpenidAllowPage />
                </OpenIDRoute>
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
