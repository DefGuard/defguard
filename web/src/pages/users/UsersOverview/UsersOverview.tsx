import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { orderBy } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import SvgIconUserAddNew from '../../../shared/components/svg/IconUserAddNew';
import { deviceBreakpoints } from '../../../shared/constants';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { Search } from '../../../shared/defguard-ui/components/Layout/Search/Search';
import { Select } from '../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import { User } from '../../../shared/types';
import { DisableMfaModal } from '../shared/modals/DisableMfaModal/DisableMfaModal';
import { UsersList } from './components/UsersList/UsersList';
import { AddUserModal } from './modals/AddUserModal/AddUserModal';
import { useAddUserModal } from './modals/AddUserModal/hooks/useAddUserModal';
import { AssignGroupsModal } from './modals/AssignGroupsModal/AssignGroupsModal';
import { useAssignGroupsModal } from './modals/AssignGroupsModal/store';

enum FilterOptions {
  ALL = 'all',
  ADMIN = 'admin',
  USERS = 'users',
}

export const UsersOverview = () => {
  const { LL, locale } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [selectedUsers, setSelectedUsers] = useState<number[]>([]);
  const openGroupsAssign = useAssignGroupsModal((s) => s.open);
  const successSubject = useAssignGroupsModal((s) => s.successSubject, shallow);

  const filterSelectOptions = useMemo(() => {
    const res: SelectOption<FilterOptions>[] = [
      {
        label: LL.usersOverview.filterLabels.all(),
        value: FilterOptions.ALL,
        key: 1,
      },
      {
        label: LL.usersOverview.filterLabels.admin(),
        value: FilterOptions.ADMIN,
        key: 2,
      },
      {
        label: LL.usersOverview.filterLabels.users(),
        value: FilterOptions.USERS,
        key: 3,
      },
    ];
    return res;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locale]);

  const renderSelectedFilter = useCallback(
    (selected: FilterOptions): SelectSelectedValue => {
      const option = filterSelectOptions.find((o) => o.value === selected);
      if (!option) throw Error("Selected value doesn't exist");
      return {
        key: option.key,
        displayValue: option.label,
      };
    },
    [filterSelectOptions],
  );

  const [selectedFilter, setSelectedFilter] = useState(FilterOptions.ALL);

  const {
    user: { getUsers },
  } = useApi();

  const { data: users, isLoading } = useQuery({
    queryKey: [QueryKeys.FETCH_USERS_LIST],
    queryFn: getUsers,
  });

  const [usersSearchValue, setUsersSearchValue] = useState('');

  const openAddUserModal = useAddUserModal((state) => state.open);

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
            .includes(usersSearchValue.toLocaleLowerCase()),
      );
    }
    if (searched.length) {
      searched = orderBy(searched, ['username'], ['asc']);
    }
    switch (selectedFilter) {
      case FilterOptions.ALL:
        break;
      case FilterOptions.ADMIN:
        searched = searched.filter((user) => user.is_admin);
        break;
      case FilterOptions.USERS:
        searched = searched.filter((user) => !user.is_admin);
        break;
    }
    return searched;
  }, [selectedFilter, users, usersSearchValue]);

  const handleUserSelect = useCallback(
    (id: number) => {
      if (selectedUsers.includes(id)) {
        setSelectedUsers((selected) => selected.filter((i) => i !== id));
      } else {
        setSelectedUsers((s) => [...s, id]);
      }
    },
    [selectedUsers],
  );

  const handleSelectAll = useCallback(() => {
    if (users) {
      if (users.length !== selectedUsers.length) {
        setSelectedUsers(users.map((u) => u.id));
      } else {
        setSelectedUsers([]);
      }
    }
  }, [users, selectedUsers, setSelectedUsers]);

  useEffect(() => {
    if (breakpoint !== 'desktop' && selectedFilter !== FilterOptions.ALL) {
      setSelectedFilter(FilterOptions.ALL);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [breakpoint]);

  useEffect(() => {
    const sub = successSubject.subscribe(() => {
      setSelectedUsers([]);
    });
    return () => {
      sub?.unsubscribe();
    };
  }, [successSubject]);

  return (
    <section id="users-overview">
      {breakpoint === 'desktop' && (
        <header>
          <h1>{LL.usersOverview.pageTitle()}</h1>
          <Search
            placeholder={LL.usersOverview.search.placeholder()}
            className="users-search"
            initialValue={usersSearchValue}
            debounceTiming={500}
            onDebounce={(value: string) => setUsersSearchValue(value)}
          />
        </header>
      )}
      <motion.section className="actions">
        <div className="items-count">
          <span>{LL.usersOverview.usersCount()}</span>
          <div className="count" data-testid="users-count">
            <span>{users && users.length ? users.length : 0}</span>
          </div>
        </div>
        <div className="controls">
          {selectedUsers.length > 0 && (
            <Button
              size={ButtonSize.SMALL}
              styleVariant={ButtonStyleVariant.PRIMARY}
              text="Assign group to selected users"
              onClick={() => openGroupsAssign(selectedUsers)}
            />
          )}
          {breakpoint === 'desktop' && (
            <Select
              sizeVariant={SelectSizeVariant.SMALL}
              searchable={false}
              renderSelected={renderSelectedFilter}
              selected={selectedFilter}
              options={filterSelectOptions}
              onChangeSingle={(filter) => setSelectedFilter(filter)}
            />
          )}
          <Button
            className="add-item"
            onClick={openAddUserModal}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconUserAddNew />}
            text={breakpoint === 'desktop' ? LL.usersOverview.addNewUser() : undefined}
            data-testid="add-user"
          />
        </div>
        {breakpoint !== 'desktop' ? (
          <Search
            placeholder={LL.usersOverview.search.placeholder()}
            className="users-search"
            debounceTiming={500}
            onDebounce={(value) => setUsersSearchValue(value)}
            initialValue={usersSearchValue}
          />
        ) : null}
      </motion.section>
      {!isLoading && filteredUsers && filteredUsers.length > 0 && (
        <UsersList
          users={filteredUsers}
          selectedUsers={selectedUsers}
          allSelected={users && selectedUsers.length === users.length}
          onSelectAll={handleSelectAll}
          onUserSelect={handleUserSelect}
        />
      )}
      {isLoading && (
        <div className="list-loader">
          <LoaderSpinner size={180} />
        </div>
      )}
      <AddUserModal />
      <AssignGroupsModal />
      <DisableMfaModal />
    </section>
  );
};
