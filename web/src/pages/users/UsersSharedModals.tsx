import { AddAuthenticationKeyModal } from './shared/modals/AddAuthenticationKeyModal/AddAuthenticationKeyModal';
import { ChangePasswordModal } from './shared/modals/ChangeUserPasswordModal/ChangeUserPasswordModal';
import { DeleteUserModal } from './shared/modals/DeleteUserModal/DeleteUserModal';

/***
 * Shared modals for /users and /me
 ***/
export const UsersSharedModals = () => {
  return (
    <>
      <ChangePasswordModal />
      <DeleteUserModal />
      <AddAuthenticationKeyModal />
    </>
  );
};
