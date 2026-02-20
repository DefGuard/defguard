import { Page } from '../../shared/components/Page/Page';
import './style.scss';
import { Suspense } from 'react';
import { m } from '../../paraglide/messages';
import { AddAuthKeyModal } from '../../shared/components/modals/AddAuthKeyModal/AddAuthKeyModal';
import { ChangePasswordModal } from '../../shared/components/modals/ChangePasswordModal/ChangePasswordModal';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { AddNewDeviceModal } from './modals/AddNewDeviceModal/AddNewDeviceModal';
import { AddUserModal } from './modals/AddUserModal/AddUserModal';
import { AssignUserIPModal } from './modals/AssignUserIPModal/AssignUserIPModal';
import { AssignUsersToGroupsModal } from './modals/AssignUsersToGroupsModal/AssignUsersToGroupsModal';
import { EditUserModal } from './modals/EditUserModal/EditUserModal';
import { EnrollmentTokenModal } from './modals/EnrollmentTokenModal/EnrollmentTokenModal';
import { UsersTable } from './UsersTable';

export const UsersOverviewPage = () => {
  return (
    <>
      <Page title={m.users_title()} id="users-overview-page">
        <Suspense fallback={<TableSkeleton />}>
          <TablePageLayout>
            <UsersTable />
          </TablePageLayout>
        </Suspense>
      </Page>
      <AddNewDeviceModal />
      <AddUserModal />
      <EditUserModal />
      <EnrollmentTokenModal />
      <AddAuthKeyModal />
      <ChangePasswordModal />
      <AssignUsersToGroupsModal />
      <AssignUserIPModal />
    </>
  );
};
