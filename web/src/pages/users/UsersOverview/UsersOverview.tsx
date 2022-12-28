import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { orderBy } from 'lodash-es';
import { useEffect, useState } from 'react';
import { useMemo } from 'react';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import LoaderSpinner from '../../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import { Search } from '../../../shared/components/layout/Search/Search';
import {
  Select,
  SelectOption,
} from '../../../shared/components/layout/Select/Select';
import SvgIconUserAddNew from '../../../shared/components/svg/IconUserAddNew';
import { deviceBreakpoints } from '../../../shared/constants';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import { User } from '../../../shared/types';
import { UsersList } from './components/UsersList/UsersList';
import AddUserModal from './modals/AddUserModal/AddUserModal';

enum FilterOptions {
  ALL = 'all',
  ADMIN = 'admin',
  USERS = 'users',
}

const filterSelectOptions: SelectOption<FilterOptions>[] = [
  {
    label: 'All users',
    value: FilterOptions.ALL,
    key: 1,
  },
  {
    label: 'Admins only',
    value: FilterOptions.ADMIN,
    key: 2,
  },
  { label: 'Users only', value: FilterOptions.USERS, key: 3 },
];

export const UsersOverview = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [selectedFilter, setSelectedFilter] = useState(filterSelectOptions[0]);

  const {
    user: { getUsers },
  } = useApi();
  const { data: users, isLoading } = useQuery(
    [QueryKeys.FETCH_USERS],
    getUsers
  );

  const [usersSearchValue, setUsersSearchValue] = useState('');

  const setUserAddModalState = useModalStore((state) => state.setAddUserModal);

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
      searched = orderBy(searched, ['username'], ['asc']);
    }
    switch (selectedFilter.value) {
      case FilterOptions.ALL:
        break;
      case FilterOptions.ADMIN:
        searched = searched.filter((user) => user.groups.includes('admin'));
        break;
      case FilterOptions.USERS:
        searched = searched.filter((user) => !user.groups.includes('admin'));
        break;
    }
    return searched;
  }, [selectedFilter.value, users, usersSearchValue]);

  useEffect(() => {
    if (
      breakpoint !== 'desktop' &&
      selectedFilter.value !== FilterOptions.ALL
    ) {
      setSelectedFilter(filterSelectOptions[0]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [breakpoint]);

  return (
    <section id="users-overview">
      {breakpoint === 'desktop' && (
        <motion.header>
          <h1>Users</h1>
          <Search
            placeholder="Find users"
            className="users-search"
            initialValue={usersSearchValue}
            debounceTiming={500}
            onDebounce={(value: string) => setUsersSearchValue(value)}
          />
        </motion.header>
      )}
      <motion.section className="actions">
        <div className="items-count">
          <span>All users</span>
          <div className="count" data-test="users-count">
            <span>{users && users.length ? users.length : 0}</span>
          </div>
        </div>
        <div className="controls">
          {breakpoint === 'desktop' && (
            <Select
              multi={false}
              searchable={false}
              selected={selectedFilter}
              options={filterSelectOptions}
              onChange={(option) => {
                if (option && !Array.isArray(option)) {
                  setSelectedFilter(option);
                }
              }}
            />
          )}
          <Button
            className="add-item"
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
            debounceTiming={500}
            onDebounce={(value) => setUsersSearchValue(value)}
            initialValue={usersSearchValue}
          />
        ) : null}
      </motion.section>
      {!isLoading && filteredUsers && filteredUsers.length > 0 && (
        <UsersList users={filteredUsers} />
      )}
      {isLoading && (
        <div className="list-loader">
          <LoaderSpinner size={180} />
        </div>
      )}
      <AddUserModal />
    </section>
  );
};
