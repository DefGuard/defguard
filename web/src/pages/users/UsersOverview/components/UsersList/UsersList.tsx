import './style.scss';

import { useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router';
import useBreakpoint from 'use-breakpoint';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import UserInitials from '../../../../../shared/components/layout/UserInitials/UserInitials';
import {
  ListHeader,
  ListRowCell,
  ListSortDirection,
  VirtualizedList,
} from '../../../../../shared/components/layout/VirtualizedList/VirtualizedList';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useNavigationStore } from '../../../../../shared/hooks/store/useNavigationStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import { User } from '../../../../../shared/types';
import { UserEditButton } from '../UserEditButton/UserEditButton';

type Props = {
  users: User[];
};

export const UsersList = ({ users }: Props) => {
  const { LL, locale } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const navigate = useNavigate();
  const setNavigationUser = useNavigationStore(
    (state) => state.setNavigationUser
  );
  const setUserProfile = useUserProfileStore((state) => state.setState);
  const currentUser = useAuthStore((state) => state.user);
  const navigateToUser = useCallback(
    (user: User) => {
      setUserProfile({ user: user });
      setNavigationUser(user);
      if (user.username === currentUser?.username) {
        navigate('/me', { replace: true });
      } else {
        navigate(`${user.username}`);
      }
    },
    [currentUser?.username, navigate, setNavigationUser, setUserProfile]
  );

  const listHeaders = useMemo((): ListHeader[] => {
    if (breakpoint !== 'desktop') {
      return [];
    }
    return [
      {
        text: LL.usersOverview.list.headers.name(),
        key: 'first_name',
      },
      {
        text: LL.usersOverview.list.headers.username(),
        key: 'username',
        sortDirection: ListSortDirection.ASC,
        active: true,
      },
      {
        text: LL.usersOverview.list.headers.phone(),
        key: 'phone',
      },
      {
        text: LL.usersOverview.list.headers.actions(),
        key: 'actions',
        sortable: false,
      },
    ];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [breakpoint, locale]);

  const onCellClick = useCallback(
    (user: User) => navigateToUser(user),
    [navigateToUser]
  );

  const listCells = useMemo((): ListRowCell<User>[] => {
    const allCells = [
      {
        key: 'userFullName',
        render: (user: User) => (
          <p className="name">
            {user.first_name && user.last_name ? (
              <UserInitials
                first_name={user.first_name}
                last_name={user.last_name}
              />
            ) : null}
            {`${user.first_name} ${user.last_name}`}
          </p>
        ),
        onClick: onCellClick,
      },
      {
        key: 'username',
        render: (user: User) => (
          <span className="username">{user.username}</span>
        ),
        onClick: onCellClick,
      },
      {
        key: 'phone',
        render: (user: User) => <span className="phone">{user.phone}</span>,
      },
      {
        key: 'userActions',
        render: (user: User) => <UserEditButton user={user} />,
      },
    ];
    if (breakpoint === 'desktop') {
      return allCells;
    }
    return [allCells[0], allCells[3]];
  }, [breakpoint, onCellClick]);

  const getListPadding = useMemo(() => {
    if (breakpoint === 'desktop') {
      return {
        left: 60,
        right: 60,
      };
    }
    return {
      left: 20,
      right: 20,
    };
  }, [breakpoint]);

  return (
    <VirtualizedList
      className="users-list"
      rowSize={70}
      data={users}
      headers={listHeaders}
      cells={listCells}
      headerPadding={{
        left: 15,
        right: 15,
      }}
      padding={getListPadding}
    />
  );
};
