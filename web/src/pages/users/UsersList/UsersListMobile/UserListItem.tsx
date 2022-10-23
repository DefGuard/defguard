import React from 'react';
import { useNavigate } from 'react-router-dom';

import Divider from '../../../../shared/components/layout/Divider/Divider';
import UserInitials from '../../../../shared/components/layout/UserInitials/UserInitials';
import SvgIconUserList from '../../../../shared/components/svg/IconUserList';
import SvgIconUserListExpanded from '../../../../shared/components/svg/IconUserListExpanded';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useNavigationStore } from '../../../../shared/hooks/store/useNavigationStore';
import { User } from '../../../../shared/types';
import UserEditButton from '../UsersListTable/UserEditButton';

interface Props {
  user: User;
  expanded: boolean;
  onChangeExpand: () => void;
}

const UserListItem: React.FC<Props> = ({ user, expanded, onChangeExpand }) => {
  const loggedUser = useAuthStore((state) => state.user);
  const navigate = useNavigate();
  const setNavigationUser = useNavigationStore(
    (state) => state.setNavigationUser
  );

  const navigateToUser = () => {
    if (loggedUser?.username === user.username) {
      navigate('/me', { replace: true });
    } else {
      setNavigationUser(user);
      navigate(`/admin/users/${user.username}`, { replace: true });
    }
  };

  return (
    <div className="user-container">
      <section className="top">
        <div
          className="collapse-icon-container"
          onClick={() => onChangeExpand()}
        >
          {expanded ? <SvgIconUserListExpanded /> : <SvgIconUserList />}
        </div>
        {user.first_name && user.last_name ? (
          <>
            <UserInitials
              first_name={user.first_name}
              last_name={user.last_name}
              onClick={navigateToUser}
            />
            <p
              className="username"
              onClick={navigateToUser}
            >{`${user.first_name} ${user.last_name}`}</p>
          </>
        ) : null}
        <UserEditButton user={user} />
      </section>
      {expanded ? (
        <>
          <Divider />
          <section className="user-details-collapse">
            <div>
              <label>Common name:</label>
              <p>{user.username}</p>
            </div>
          </section>
        </>
      ) : null}
    </div>
  );
};

export default UserListItem;
