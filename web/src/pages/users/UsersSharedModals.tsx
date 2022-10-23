import ChangeWalletModal from '../../shared/components/modals/ChangeWalletModal/ChangeWalletModal';
import { ChangePasswordModal } from './shared/modals/ChangeUserPasswordModal/ChangeUserPasswordModal';
import { DeleteUserDeviceModal } from './shared/modals/DeleteUserDeviceModal/DeleteUserDeviceModal';
import DeleteUserModal from './shared/modals/DeleteUserModal/DeleteUserModal';
import KeyDetailsModal from './shared/modals/KeyDetailsModal/KeyDetailsModal';
import KeyProvisioningModal from './shared/modals/KeyProvisioningModal/KeyProvisioningModal';
import { UserDeviceModal } from './shared/modals/UserDeviceModal/UserDeviceModal';

/***
 * Shared modals for /users and /me
 ***/
export const UsersSharedModals = () => {
  return (
    <>
      <KeyProvisioningModal />
      <DeleteUserModal />
      <ChangePasswordModal />
      <UserDeviceModal />
      <DeleteUserDeviceModal />
      <ChangeWalletModal />
      <KeyDetailsModal />
    </>
  );
};
