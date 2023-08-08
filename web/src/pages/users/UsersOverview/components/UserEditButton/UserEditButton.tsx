import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import { User } from '../../../../../shared/types';

type Props = {
  user: User;
};

export const UserEditButton = ({ user }: Props) => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const setProvisionKeyModal = useModalStore((state) => state.setProvisionKeyModal);
  const setDeleteUserModal = useModalStore((state) => state.setDeleteUserModal);
  const setChangePasswordModal = useModalStore((state) => state.setChangePasswordModal);
  const setUserProfile = useUserProfileStore((state) => state.setState);
  const currentUser = useAuthStore((state) => state.user);
  return (
    <EditButton>
      <EditButtonOption
        key="change-password"
        text={LL.usersOverview.list.editButton.changePassword()}
        onClick={() => setChangePasswordModal({ visible: true, user: user })}
      />
      <EditButtonOption
        key="edit-user"
        text={LL.usersOverview.list.editButton.edit()}
        onClick={() => {
          setUserProfile({ editMode: true });
          navigate(`/admin/users/${user.username}`, { replace: true });
        }}
      />
      <EditButtonOption
        key="provision-yubi-key"
        text={LL.usersOverview.list.editButton.provision()}
        onClick={() => setProvisionKeyModal({ visible: true, user: user })}
      />
      {user.username !== currentUser?.username && (
        <EditButtonOption
          key="delete-user"
          text={LL.usersOverview.list.editButton.delete()}
          onClick={() => setDeleteUserModal({ visible: true, user: user })}
          styleVariant={EditButtonOptionStyleVariant.WARNING}
        />
      )}
    </EditButton>
  );
};
