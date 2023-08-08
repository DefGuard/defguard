import './style.scss';

import { Navigate, Route, Routes } from 'react-router-dom';

import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { UserProfile } from './UserProfile/UserProfile';
import { UsersOverview } from './UsersOverview/UsersOverview';
import { UsersSharedModals } from './UsersSharedModals';

export const UsersPage = () => {
  return (
    <PageContainer id="users-page">
      <Routes>
        <Route path="" element={<UsersOverview />} />
        <Route path=":username/*" element={<UserProfile />} />
        <Route path="*" element={<Navigate replace to="" />} />
      </Routes>
      <UsersSharedModals />
    </PageContainer>
  );
};
