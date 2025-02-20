import { AddApiTokenModal } from './shared/modals/AddApiTokenModal/AddApiTokenModal';
import { AddAuthenticationKeyModal } from './shared/modals/AddAuthenticationKeyModal/AddAuthenticationKeyModal';
import { ChangePasswordModal } from './shared/modals/ChangeUserPasswordModal/ChangeUserPasswordModal';
import { DeleteUserModal } from './shared/modals/DeleteUserModal/DeleteUserModal';
import { ToggleUserModal } from './shared/modals/ToggleUserModal/ToggleUserModal';

/***
 * Shared modals for /users and /me
 ***/
export const UsersSharedModals = () => {
  return (
    <>
      <ChangePasswordModal />
      <DeleteUserModal />
      <ToggleUserModal />
      <AddAuthenticationKeyModal />
      <AddApiTokenModal />
    </>
  );
};
