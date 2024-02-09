import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';

import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Search } from '../../../../shared/defguard-ui/components/Layout/Search/Search';
import SvgIconPlus from '../../../../shared/defguard-ui/components/svg/IconPlus';
import useApi from '../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../shared/queries';
import { GroupsList } from '../GroupsList/GroupsList';
import { useAddGroupModal } from '../modals/AddGroupModal/useAddGroupModal';

export const GroupsManagement = () => {
  const openGroupModal = useAddGroupModal((s) => s.open);
  const {
    groups: { getGroupsInfo },
  } = useApi();
  const [search, setSearch] = useState('');

  const { data } = useQuery({
    queryKey: [QueryKeys.FETCH_GROUPS_INFO],
    queryFn: getGroupsInfo,
  });

  return (
    <div className="content-wrapper">
      <header>
        <h1>Groups Management</h1>
        <Search
          placeholder="Find"
          debounceTiming={1000}
          onDebounce={(v) => setSearch(v)}
        />
      </header>
      <div id="groups-control">
        <h3>All groups</h3>
        <div id="items-count">
          <span>{data?.length ?? 0}</span>
        </div>
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          icon={<SvgIconPlus />}
          text="Add new"
          onClick={() => openGroupModal()}
        />
      </div>
      {data && <GroupsList groups={data} search={search} />}
    </div>
  );
};
