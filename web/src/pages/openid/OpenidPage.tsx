import './style.scss';

import React from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';

import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import LoaderPage from '../loader/LoaderPage';
import DeleteClientModal from './modals/DeleteClientModal/DeleteClientModal';
import EnableClientModal from './modals/EnableClientModal/EnableClientModal';
import OpenidClientsList from './OpenidClientsList/OpenidClientsList';

const OpenidClient = React.lazy(() => import('./OpenidClient/OpenidClient'));

const OpenidPage = () => {
  return (
    <PageContainer id="openid">
      <Routes>
        <Route path="" element={<OpenidClientsList />} />
        <Route
          path=":id"
          element={
            <React.Suspense fallback={<LoaderPage />}>
              <OpenidClient />
            </React.Suspense>
          }
        />
        <Route path="*" element={<Navigate replace to="" />} />
      </Routes>
      <DeleteClientModal />
      <EnableClientModal />
    </PageContainer>
  );
};

export default OpenidPage;
