import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useCallback, useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Divider } from '../../../../../shared/defguard-ui/components/Layout/Divider/Divider';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { Search } from '../../../../../shared/defguard-ui/components/Layout/Search/Search';
import { SelectRow } from '../../../../../shared/defguard-ui/components/Layout/SelectRow/SelectRow';
import useApi from '../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../shared/queries';
import { useAssignGroupsModal } from './store';

export const AssignGroupsModal = () => {
  const isOpen = useAssignGroupsModal((s) => s.visible);
  const [close, reset] = useAssignGroupsModal((s) => [s.close, s.reset], shallow);
  return (
    <ModalWithTitle
      id="assign-groups-modal"
      title="Assign group to selected users"
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const [search, setSearch] = useState<string>('');
  const [selected, setSelected] = useState<string[]>([]);
  const {
    groups: { getGroups },
  } = useApi();

  const closeModal = useAssignGroupsModal((s) => s.close);

  const { data: groups } = useQuery({
    queryKey: [QueryKeys.FETCH_GROUPS],
    queryFn: async () => getGroups().then((res) => res.groups),
  });

  const filteredGroups = useMemo(() => {
    if (groups) {
      if (search.length > 0 && groups) {
        return groups?.filter((g) => g.toLowerCase().includes(search.toLowerCase()));
      }
      return groups;
    }
    return [];
  }, [groups, search]);

  const handleSelect = useCallback(
    (group: string) => {
      const isSelected = !isUndefined(selected.find((g) => g === group));
      if (isSelected) {
        setSelected((state) => state.filter((g) => g !== group));
      } else {
        setSelected((state) => [...state, group]);
      }
    },
    [setSelected, selected],
  );

  const handleSelectAll = useCallback(() => {
    if (groups) {
      if (selected.length !== groups.length) {
        setSelected(groups);
      } else {
        setSelected([]);
      }
    }
  }, [groups, selected.length]);

  return (
    <>
      <header>
        <p>Groups</p>
        <Search
          placeholder="Filter/Search"
          debounceTiming={1000}
          onDebounce={(searchVal) => setSearch(searchVal)}
        />
      </header>
      <div className="content">
        <div className="padding-wrapper">
          <SelectRow
            selected={selected.length === groups?.length}
            onClick={() => handleSelectAll()}
          >
            <p>Select All</p>
          </SelectRow>
          <Divider />
        </div>
        <div className="groups-container">
          <div className="scroll-wrapper">
            {filteredGroups.map((g) => (
              <SelectRow
                key={g}
                onClick={() => handleSelect(g)}
                selected={!isUndefined(selected.find((group) => g === group))}
              >
                <p>{g}</p>
              </SelectRow>
            ))}
          </div>
        </div>
      </div>
      <div className="controls">
        <Button size={ButtonSize.LARGE} text="Cancel" onClick={() => closeModal()} />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Assign groups"
          disabled={selected.length === 0}
        />
      </div>
    </>
  );
};
