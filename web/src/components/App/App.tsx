import 'react-loading-skeleton/dist/skeleton.css';
import './App.scss';

import { BrowserRouter as Router, Navigate, Route, Routes } from 'react-router-dom';

import { AddDevicePage } from '../../pages/addDevice/AddDevicePage';
import { OpenidAllowPage } from '../../pages/allow/OpenidAllowPage';
import { AuthPage } from '../../pages/auth/AuthPage';
import { EnrollmentPage } from '../../pages/enrollment/EnrollmentPage';
import { GroupsPage } from '../../pages/groups/GroupsPage';
import { NetworkPage } from '../../pages/network/NetworkPage';
import { OpenidClientsListPage } from '../../pages/openid/OpenidClientsListPage/OpenidClientsListPage';
import { OverviewPage } from '../../pages/overview/OverviewPage';
import { ProvisionersPage } from '../../pages/provisioners/ProvisionersPage';
import { SettingsPage } from '../../pages/settings/SettingsPage';
import { SupportPage } from '../../pages/support/SupportPage';
import { UserProfile } from '../../pages/users/UserProfile/UserProfile';
import { UsersPage } from '../../pages/users/UsersPage';
import { UsersSharedModals } from '../../pages/users/UsersSharedModals';
import { WebhooksListPage } from '../../pages/webhooks/WebhooksListPage';
import { WizardPage } from '../../pages/wizard/WizardPage';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { UpdateNotificationModal } from '../../shared/components/modals/UpdateNotificationModal/UpdateNotificationModal';
import { ProtectedRoute } from '../../shared/components/Router/Guards/ProtectedRoute/ProtectedRoute';
import { ToastManager } from '../../shared/defguard-ui/components/Layout/ToastManager/ToastManager';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import { Navigation } from '../Navigation/Navigation';
import { AddStandaloneDeviceModal } from '../../pages/overview/modals/AddStandaloneDeviceModal/AddStandaloneDeviceModal';

const App = () => {
  const currentUser = useAuthStore((state) => state.user);
  const isAdmin = useAuthStore((state) => state.isAdmin);
  return (
    <>
      <div id="app">
        <Router>
          <Routes>
            <Route
              path="add-device"
              element={
                <ProtectedRoute>
                  <AddDevicePage />
                </ProtectedRoute>
              }
            />
            <Route
              path="support/*"
              element={
                <ProtectedRoute>
                  <SupportPage />
                </ProtectedRoute>
              }
            />
            <Route path="auth/*" element={<AuthPage />} />
            <Route path="admin/*">
              <Route index element={<Navigate to="users" />} />
              <Route
                path="groups/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <GroupsPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="enrollment/*"
                element={
                  <ProtectedRoute allowedGroups={['admin']}>
                    <EnrollmentPage />
                  </ProtectedRoute>
                }
              />
              <Route
                path="network/*"
                element={
                  <ProtectedRoute
                    allowedGroups={['admin']}
                    moduleRequired="wireguard_enabled"
                  >
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
                <ProtectedRoute allowUnauthorized moduleRequired="openid_enabled">
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
          <Navigation />
          <UpdateNotificationModal />
          <AddStandaloneDeviceModal />
        </Router>
      </div>
      <ToastManager />
    </>
  );
};

export default App;
