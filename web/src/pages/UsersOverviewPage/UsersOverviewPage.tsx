import { useQuery } from '@tanstack/react-query';
import { LayoutGrid } from '../../shared/components/LayoutGrid/LayoutGrid';
import { Page } from '../../shared/components/Page/Page';
import './style.scss';
import { m } from '../../paraglide/messages';
import { AddAuthKeyModal } from '../../shared/components/modals/AddAuthKeyModal/AddAuthKeyModal';
import { ChangePasswordModal } from '../../shared/components/modals/ChangePasswordModal/ChangePasswordModal';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { getUsersQueryOptions } from '../../shared/query';
import { AddUserModal } from './modals/AddUserModal/AddUserModal';
import { EditUserModal } from './modals/EditUserModal/EditUserModal';
import { UsersTable } from './UsersTable';

export const UsersOverviewPage = () => {
  const { data: users } = useQuery(getUsersQueryOptions);
  return (
    <>
      <Page title={m.users_title()} id="users-overview-page">
        <LayoutGrid>{isPresent(users) && <UsersTable users={users} />}</LayoutGrid>
      </Page>
      <AddUserModal />
      <EditUserModal />
      <AddAuthKeyModal />
      <ChangePasswordModal />
    </>
  );
};
