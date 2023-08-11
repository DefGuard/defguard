import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { orderBy } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

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
  SelectSizeVariant,
} from '../../../shared/defguard-ui/components/Layout/Select/types';
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

export const UsersOverview = () => {
  const { LL, locale } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

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

  const [selectedFilter, setSelectedFilter] = useState(FilterOptions.ALL);

  const {
    user: { getUsers },
  } = useApi();
  const { data: users, isLoading } = useQuery([QueryKeys.FETCH_USERS_LIST], getUsers);

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
        searched = searched.filter((user) => user.groups.includes('admin'));
        break;
      case FilterOptions.USERS:
        searched = searched.filter((user) => !user.groups.includes('admin'));
        break;
    }
    return searched;
  }, [selectedFilter, users, usersSearchValue]);

  useEffect(() => {
    if (breakpoint !== 'desktop' && selectedFilter !== FilterOptions.ALL) {
      setSelectedFilter(FilterOptions.ALL);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [breakpoint]);

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
          {breakpoint === 'desktop' && (
            <Select
              sizeVariant={SelectSizeVariant.SMALL}
              searchable={false}
              selected={selectedFilter}
              options={filterSelectOptions}
              onChangeSingle={(filter) => setSelectedFilter(filter)}
            />
          )}
          <Button
            className="add-item"
            onClick={() => setUserAddModalState({ visible: true })}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconUserAddNew />}
            text={LL.usersOverview.addNewUser()}
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
