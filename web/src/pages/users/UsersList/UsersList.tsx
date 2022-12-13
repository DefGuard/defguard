import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { orderBy } from 'lodash-es';
import React, { useState } from 'react';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import Select from 'react-select';
import { Column } from 'react-table';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import Search from '../../../shared/components/layout/Search/Search';
// import Avatar from '../../../shared/components/layout/Avatar/Avatar';
import UserInitials from '../../../shared/components/layout/UserInitials/UserInitials';
import SvgIconCheckmarkGreen from '../../../shared/components/svg/IconCheckmarkGreen';
import SvgIconUserAddNew from '../../../shared/components/svg/IconUserAddNew';
import { deviceBreakpoints } from '../../../shared/constants';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import { useNavigationStore } from '../../../shared/hooks/store/useNavigationStore';
import { useUserProfileStore } from '../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import { User } from '../../../shared/types';
import { standardVariants } from '../../../shared/variants';
import AddUserModal from './AddUserModal/AddUserModal';
import UsersListMobile from './UsersListMobile/UsersListMobile';
import UsersListTable from './UsersListTable/UsersListTable';

const UsersList: React.FC = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const navigate = useNavigate();
  const {
    user: { getUsers },
  } = useApi();
  const { data: users, isLoading } = useQuery(
    [QueryKeys.FETCH_USERS],
    getUsers
  );

  const [usersSearchValue, setUsersSearchValue] = useState('');
  const setNavigationUser = useNavigationStore(
    (state) => state.setNavigationUser
  );
  const setUserProfile = useUserProfileStore((state) => state.setState);
  const setUserAddModalState = useModalStore((state) => state.setAddUserModal);
  const currentUser = useAuthStore((state) => state.user);

  const navigateToUser = (user: User) => {
    setUserProfile({ user: user });
    setNavigationUser(user);
    if (user.username === currentUser?.username) {
      navigate('/me', { replace: true });
    } else {
      navigate(`${user.username}`);
    }
  };

  const tableColumns: Column<User>[] = useMemo(
    () => [
      {
        Header: 'User name',
        accessor: 'first_name',
        Cell: ({ row }) => {
          return (
            <p className="name" onClick={() => navigateToUser(row.original)}>
              {row.original.first_name && row.original.last_name ? (
                <UserInitials
                  first_name={row.original.first_name}
                  last_name={row.original.last_name}
                />
              ) : null}
              {row.original.first_name + ' ' + row.original.last_name}
            </p>
          );
        },
      },
      {
        Header: 'Common name',
        accessor: 'username',
        Cell: (cell) => <p className="username">{cell.value}</p>,
      },
      // {
      //   Header: 'Devices',
      //   accessor: 'devices',
      //   Cell: () => (
      //     <div className="devices">
      //       <Avatar />
      //       <Avatar active={false} />
      //       <div className="avatar-icon">
      //         <span>+2</span>
      //       </div>
      //     </div>
      //   ),
      // },
      // {
      //   Header: 'Last connected',
      //   accessor: 'lastConnected',
      //   Cell: () => (
      //     <p className="connection">
      //       <span className="icon connected"></span>7min
      //     </p>
      //   ),
      // },
      // {
      //   Header: 'Last Location',
      //   accessor: 'lastLocation',
      //   Cell: () => (
      //     <div className="locations">
      //       <span className="tag">Szczecin</span>
      //       <span className="tag">169.254.0.0</span>
      //     </div>
      //   ),
      // },
      // {
      //   Header: 'Last connections',
      //   accessor: 'lastLocations',
      //   Cell: () => (
      //     <div className="connection-history">
      //       <span className="avatar small">ZK</span>
      //       <span className="avatar small">AP</span>
      //       <span className="avatar small">RO</span>
      //     </div>
      //   ),
      // },
      {
        Header: 'Status',
        accessor: 'status',
        Cell: () => (
          <p className="status">
            <SvgIconCheckmarkGreen />
            Active
          </p>
        ),
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    []
  );

  const filteredUsers = useMemo(() => {
    if (!users || (users && !users.length)) {
      return [];
    }
    let searched: User[] = [];
    if (users) {
      searched = users.filter(
        (user) =>
          user.username
            .toLocaleLowerCase()
            .includes(usersSearchValue.toLocaleLowerCase()) ||
          user.first_name
            ?.toLocaleLowerCase()
            .includes(usersSearchValue.toLocaleLowerCase()) ||
          user.last_name
            ?.toLocaleLowerCase()
            .includes(usersSearchValue.toLocaleLowerCase())
      );
    }
    if (searched.length) {
      return orderBy(searched, ['username'], ['asc']);
    }
    return searched;
  }, [users, usersSearchValue]);

  return (
    <section id="users-list">
      {breakpoint === 'desktop' && (
        <motion.header
          variants={standardVariants}
          initial="hidden"
          animate="show"
        >
          <h1>Users</h1>
          <Search
            placeholder="Find users"
            className="users-search"
            value={usersSearchValue}
            onChange={(e) => setUsersSearchValue(e.target.value)}
          />
        </motion.header>
      )}
      <motion.section
        className="actions"
        variants={standardVariants}
        initial="hidden"
        animate="show"
      >
        <div className="users-count">
          <span>All users</span>
          <div className="count" data-test="users-count">
            <span>{users && users.length ? users.length : 0}</span>
          </div>
        </div>
        <div className="table-controls">
          {breakpoint === 'desktop' ? (
            <Select
              placeholder="All users"
              options={[{ value: 'all', label: 'All users' }]}
              className="custom-select"
              classNamePrefix="rs"
            />
          ) : null}
          <Button
            className="add-user"
            onClick={() => setUserAddModalState({ visible: true })}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconUserAddNew />}
            text="Add new"
          />
        </div>
        {breakpoint !== 'desktop' ? (
          <Search
            placeholder="Find users"
            className="users-search"
            value={usersSearchValue}
            onChange={(e) => setUsersSearchValue(e.target.value)}
          />
        ) : null}
      </motion.section>
      {breakpoint === 'desktop' ? (
        <section className="users-table">
          {users && users.length && !isLoading ? (
            <UsersListTable
              data={filteredUsers}
              columns={tableColumns}
              navigateToUser={navigateToUser}
            />
          ) : null}
        </section>
      ) : null}
      {breakpoint !== 'desktop' && !isLoading && users ? (
        <UsersListMobile users={filteredUsers} />
      ) : null}
      <AddUserModal />
    </section>
  );
};

export default UsersList;
