import type { PropsWithChildren } from 'react';
import { Route, Routes } from 'react-router';
import { AclCreateDataProvider } from './AclCreateDataProvider';
import { AlcCreatePage } from './AclCreatePage/AclCreatePage';
import { AclIndexPage } from './AclIndexPage/AclIndexPage';
import { AclCreateTrackedProvider } from './acl-context';

const AclProvide = ({ children }: PropsWithChildren) => {
  return (
    <AclCreateTrackedProvider>
      <AclCreateDataProvider>{children}</AclCreateDataProvider>
    </AclCreateTrackedProvider>
  );
};

export const AclRoutes = () => {
  return (
    <Routes>
      <Route
        index
        element={
          <AclProvide>
            <AclIndexPage />
          </AclProvide>
        }
      />
      <Route
        path="form"
        element={
          <AclProvide>
            <AlcCreatePage />
          </AclProvide>
        }
      />
    </Routes>
  );
};
