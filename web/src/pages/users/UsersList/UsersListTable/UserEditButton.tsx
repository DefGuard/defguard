import { useNavigate } from 'react-router-dom';

import { EditButton } from '../../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../../shared/components/layout/EditButton/EditButtonOption';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useNavigationStore } from '../../../../shared/hooks/store/useNavigationStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import { User } from '../../../../shared/types';

interface Props {
  user: User;
}

const UserEditButton: React.FC<Props> = ({ user }) => {
  const navigate = useNavigate();
  const setProvisionKeyModal = useModalStore(
    (state) => state.setProvisionKeyModal
  );
  const setDeleteUserModal = useModalStore((state) => state.setDeleteUserModal);
  const setChangePasswordModal = useModalStore(
    (state) => state.setChangePasswordModal
  );
  const setUserProfile = useUserProfileV2Store((state) => state.setState);
  const setNavigationUser = useNavigationStore(
    (state) => state.setNavigationUser
  );
  const currentUser = useAuthStore((state) => state.user);
  return (
    <EditButton>
      <EditButtonOption
        key="change-password"
        text="Change password"
        onClick={() => setChangePasswordModal({ visible: true, user: user })}
      />
      <EditButtonOption
        key="edit-user"
        text="Edit account"
        onClick={() => {
          navigate(`/admin/users/${user.username}/edit`, { replace: true });
          setUserProfile({ user: user });
          setNavigationUser(user);
        }}
      />
      <EditButtonOption
        key="provision-yubi-key"
        text="Provision YubiKey"
        onClick={() => setProvisionKeyModal({ visible: true, user: user })}
      />
      {user.username !== currentUser?.username && (
        <EditButtonOption
          key="delete-user"
          text="Delete account"
          onClick={() => setDeleteUserModal({ visible: true, user: user })}
          styleVariant={EditButtonOptionStyleVariant.WARNING}
        />
      )}
    </EditButton>
  );
};

export default UserEditButton;
