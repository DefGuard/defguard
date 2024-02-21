import './style.scss';

import { useMemo } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { CheckBox } from '../../../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import {
  ListHeader,
  ListSortDirection,
} from '../../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import { User } from '../../../../../shared/types';
import { UserListRow } from './components/UserListRow';
import { UserGroupsListModal } from './modals/UserGroupsListModal/UserGroupsListModal';

type Props = {
  users: User[];
  onUserSelect: (id: User['id']) => void;
  onSelectAll: () => void;
  allSelected?: boolean;
  selectedUsers: User['id'][];
};

export const UsersList = ({
  users,
  selectedUsers,
  onUserSelect,
  onSelectAll,
  allSelected = false,
}: Props) => {
  const { LL, locale } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const listHeaders = useMemo((): ListHeader[] => {
    if (breakpoint !== 'desktop') {
      return [];
    }
    return [
      {
        key: 'select-all',
        text: '',
        customRender: () => (
          <div className="header" key="select-all">
            <CheckBox value={allSelected} onChange={() => onSelectAll()} />
          </div>
        ),
      },
      {
        text: LL.usersOverview.list.headers.name(),
        key: 'user-name',
        sortDirection: ListSortDirection.DESC,
        active: true,
      },
      {
        text: LL.usersOverview.list.headers.username(),
        key: 'username',
        active: false,
      },
      {
        text: LL.usersOverview.list.headers.phone(),
        key: 'phone',
        sortable: false,
      },
      {
        text: 'Groups',
        key: 'groups',
        sortable: false,
      },
      {
        text: LL.usersOverview.list.headers.actions(),
        key: 'actions',
        sortable: false,
      },
    ];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [breakpoint, locale, onSelectAll, allSelected]);

  const getListPadding = useMemo(() => {
    return {
      left: 70,
      right: 70,
    };
  }, []);

  return (
    <>
      <VirtualizedList
        className="users-list"
        rowSize={70}
        data={users}
        headers={listHeaders}
        headerPadding={{
          left: 15,
          right: 15,
        }}
        padding={getListPadding}
        customRowRender={(user) => (
          <UserListRow
            selected={selectedUsers.includes(user.id)}
            onSelect={onUserSelect}
            user={user}
            key={user.id}
          />
        )}
      />
      <UserGroupsListModal />
    </>
  );
};
