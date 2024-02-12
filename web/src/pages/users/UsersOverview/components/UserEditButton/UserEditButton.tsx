import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import { User } from '../../../../../shared/types';
import { useAddAuthorizationKeyModal } from '../../../shared/modals/AddAuthenticationKeyModal/useAddAuthorizationKeyModal';
import { useAddUserModal } from '../../modals/AddUserModal/hooks/useAddUserModal';
import { ResetPasswordButton } from './ResetPasswordButton';

type Props = {
  user: User;
};

export const UserEditButton = ({ user }: Props) => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const setDeleteUserModal = useModalStore((state) => state.setDeleteUserModal);
  const setChangePasswordModal = useModalStore((state) => state.setChangePasswordModal);
  const setUserProfile = useUserProfileStore((state) => state.setState);
  const setAddUserModal = useAddUserModal((state) => state.setState);
  const openAddAuthorizationKeyModal = useAddAuthorizationKeyModal((s) => s.open);
  const currentUser = useAuthStore((state) => state.user);
  const networkPresent = useAppStore((state) => state.appInfo?.network_present);
  return (
    <EditButton>
      {user.username !== currentUser?.username && (
        <EditButtonOption
          key="change-password"
          text={LL.usersOverview.list.editButton.changePassword()}
          onClick={() => setChangePasswordModal({ visible: true, user })}
        />
      )}
      <ResetPasswordButton user={user} />
      <EditButtonOption
        key="edit-user"
        text={LL.usersOverview.list.editButton.edit()}
        onClick={() => {
          setUserProfile({ editMode: true });
          navigate(`/admin/users/${user.username}`, { replace: true });
        }}
      />
      <EditButtonOption
        key="add-authorization-ssh"
        text={LL.usersOverview.list.editButton.addSSH()}
        onClick={() =>
          openAddAuthorizationKeyModal({
            user,
            selectedMode: 'ssh',
          })
        }
      />
      <EditButtonOption
        key="add-authorization-gpg"
        text={LL.usersOverview.list.editButton.addGPG()}
        onClick={() =>
          openAddAuthorizationKeyModal({
            user,
            selectedMode: 'gpg',
          })
        }
      />
      <EditButtonOption
        key="add-authorization-yubikey"
        text={LL.usersOverview.list.editButton.addYubikey()}
        onClick={() =>
          openAddAuthorizationKeyModal({
            user,
            selectedMode: 'yubikey',
          })
        }
      />
      {user.is_active === true && (
        <EditButtonOption
          disabled={!networkPresent}
          key="start-dekstop-activation"
          text={LL.usersOverview.list.editButton.activateDesktop()}
          onClick={() =>
            setAddUserModal({
              visible: true,
              step: 1,
              user: user,
              desktop: true,
            })
          }
        />
      )}
      {!user.is_active && (
        <EditButtonOption
          key="start-enrollment"
          text={LL.usersOverview.list.editButton.startEnrollment()}
          onClick={() =>
            setAddUserModal({
              visible: true,
              step: 1,
              user: user,
            })
          }
        />
      )}
      {user.username !== currentUser?.username && (
        <EditButtonOption
          key="delete-user"
          text={LL.usersOverview.list.editButton.delete()}
          onClick={() => setDeleteUserModal({ visible: true, user })}
          styleVariant={EditButtonOptionStyleVariant.WARNING}
        />
      )}
    </EditButton>
  );
};
