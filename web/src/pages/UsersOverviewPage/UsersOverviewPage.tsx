import { useQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import './style.scss';
import { m } from '../../paraglide/messages';
import { AddAuthKeyModal } from '../../shared/components/modals/AddAuthKeyModal/AddAuthKeyModal';
import { ChangePasswordModal } from '../../shared/components/modals/ChangePasswordModal/ChangePasswordModal';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getUsersOverviewQueryOptions } from '../../shared/query';
import { AddUserModal } from './modals/AddUserModal/AddUserModal';
import { AssignUsersToGroupsModal } from './modals/AssignUsersToGroupsModal/AssignUsersToGroupsModal';
import { EditUserModal } from './modals/EditUserModal/EditUserModal';
import { EnrollmentTokenModal } from './modals/EnrollmentTokenModal/EnrollmentTokenModal';
import { UsersTable } from './UsersTable';

export const UsersOverviewPage = () => {
  const { data: users } = useQuery(getUsersOverviewQueryOptions);
  return (
    <>
      <Page title={m.users_title()} id="users-overview-page">
        <TablePageLayout>
          {isPresent(users) && <UsersTable users={users} />}
        </TablePageLayout>
      </Page>
      <AddUserModal />
      <EditUserModal />
      <EnrollmentTokenModal />
      <AddAuthKeyModal />
      <ChangePasswordModal />
      <AssignUsersToGroupsModal />
    </>
  );
};
