import { Route, Routes } from 'react-router';

import { AclCreateTrackedProvider } from './acl-context';
import { AclCreateDataProvider } from './AclCreateDataProvider';
import { AlcCreatePage } from './AclCreatePage/AclCreatePage';
import { AclIndexPage } from './AclIndexPage/AclIndexPage';

export const AclRoutes = () => {
  return (
    <Routes>
      <Route index element={<AclIndexPage />} />
      <Route
        path="create"
        element={
          <AclCreateTrackedProvider>
            <AclCreateDataProvider>
              <AlcCreatePage />
            </AclCreateDataProvider>
          </AclCreateTrackedProvider>
        }
      />
    </Routes>
  );
};
