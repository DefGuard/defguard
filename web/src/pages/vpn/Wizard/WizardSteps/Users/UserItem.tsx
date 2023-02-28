import React from 'react';

import SvgIconPopupClose from '../../../../../shared/components/svg/IconPopupClose';
import { User } from '../../types/types';
import { useWizardStore } from '../store';

type UserItemType = {
  user: User;
};

const UserItem: React.FC<UserItemType> = ({ user }) => {
  const removeUser = useWizardStore((state) => state.removeUser);

  const getInitials = (userName: string): string => {
    if (userName) {
      const match = userName.match(/\b(\w)/g);
      if (match) {
        return match.join('').toUpperCase();
      }
    }
    return '';
  };

  return (
    <li>
      <div className="initials">
        <span>{getInitials(user.userName)}</span>
      </div>
      <div className="user-name-wrapper">
        <div className="primary">{user.userName}</div>
        <p className="secondary">
          {user.email}
          {user.locations && user.locations.length ? (
            <span>/ {user.locations.map((l) => l.name).join(', ')}</span>
          ) : null}
        </p>
      </div>
      <button className="icon-button" onClick={() => removeUser(user)}>
        <SvgIconPopupClose />
      </button>
    </li>
  );
};

export default UserItem;
