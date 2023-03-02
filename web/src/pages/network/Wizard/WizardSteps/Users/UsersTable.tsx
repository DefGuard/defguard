import React from 'react';
import { useTranslation } from 'react-i18next';

import MessageBox from '../../../../../shared/components/layout/MessageBox/MessageBox';
import { useWizardStore } from '../store';
import UserItem from './UserItem';

const UsersTable: React.FC = () => {
  const users = useWizardStore((state) => state.users);

  const { t } = useTranslation('en');
  return (
    <div className="users-table">
      <h2>{t('wizard.users.usersTable.label')}</h2>
      {users.length ? (
        <ul>
          {users.map((user) => (
            <UserItem key={user.email} user={user} />
          ))}
        </ul>
      ) : (
        <MessageBox message={t('wizard.users.usersTable.noUsers')} />
      )}
    </div>
  );
};

export default UsersTable;
