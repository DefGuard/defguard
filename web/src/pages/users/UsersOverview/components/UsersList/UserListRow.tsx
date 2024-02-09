import { useCallback } from 'react';
import { useNavigate } from 'react-router';

import { CheckBox } from '../../../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import { UserInitials } from '../../../../../shared/defguard-ui/components/Layout/UserInitials/UserInitials';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import { User } from '../../../../../shared/types';
import { UserEditButton } from '../UserEditButton/UserEditButton';

type Props = {
  user: User;
  onSelect: (id: User['id']) => void;
  selected?: boolean;
};

export const UserListRow = ({ user, onSelect, selected = false }: Props) => {
  const navigate = useNavigate();
  const currentUser = useAuthStore((state) => state.user);
  const resetUserProfile = useUserProfileStore((s) => s.reset);

  const navigateToUser = useCallback(
    (user: User) => {
      resetUserProfile();
      if (user.username === currentUser?.username) {
        navigate('/me', { replace: true });
      } else {
        navigate(`${user.username}`);
      }
    },
    [currentUser?.username, navigate, resetUserProfile],
  );

  return (
    <div className="users-list-row" data-testid={`user-${user.id}`}>
      <div className="select-cell" onClick={() => onSelect(user.id)}>
        <CheckBox value={selected} />
      </div>
      <div className="name-cell" onClick={() => navigateToUser(user)}>
        {user.first_name && user.last_name ? (
          <UserInitials first_name={user.first_name} last_name={user.last_name} />
        ) : null}
        <span>{`${user.first_name} ${user.last_name}`}</span>
      </div>
      <div className="username-cell">
        <span>{user.username}</span>
      </div>
      <div className="user-phone-cell">
        <span>{user.phone}</span>
      </div>
      <div className="groups-cell">
        {user.groups.map((g) => (
          <div className="group" key={g}>
            <span>{g}</span>
          </div>
        ))}
      </div>
      <div className="user-edit-cell">
        <UserEditButton user={user} />
      </div>
    </div>
  );
};
