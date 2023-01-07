import { ChangePasswordModal } from './shared/modals/ChangeUserPasswordModal/ChangeUserPasswordModal';
import { DeleteUserModal } from './shared/modals/DeleteUserModal/DeleteUserModal';
import { KeyProvisioningModal } from './shared/modals/KeyProvisioningModal/KeyProvisioningModal';

/***
 * Shared modals for /users and /me
 ***/
export const UsersSharedModals = () => {
  return (
    <>
      <ChangePasswordModal />
      <DeleteUserModal />
      <KeyProvisioningModal />
    </>
  );
};
