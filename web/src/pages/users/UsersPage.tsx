import './style.scss';

import React from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';

import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { UserProfileV2 } from './UserProfileV2/UserProfileV2';
import UsersList from './UsersList/UsersList';
import { UsersSharedModals } from './UsersSharedModals';

const UsersPage: React.FC = () => {
  return (
    <PageContainer id="users">
      <Routes>
        <Route path="" element={<UsersList />} />
        <Route path=":username/*" element={<UserProfileV2 />} />
        <Route path="*" element={<Navigate replace to="" />} />
      </Routes>
      <UsersSharedModals />
    </PageContainer>
  );
};

export default UsersPage;
