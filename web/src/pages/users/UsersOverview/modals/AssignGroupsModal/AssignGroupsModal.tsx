import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useCallback, useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
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
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { invalidateMultipleQueries } from '../../../../../shared/utils/invalidateMultipleQueries';
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

const toInvalidate = [
  QueryKeys.FETCH_GROUPS,
  QueryKeys.FETCH_GROUPS_INFO,
  QueryKeys.FETCH_USERS_LIST,
];

const ModalContent = () => {
  const [search, setSearch] = useState<string>('');
  const [selected, setSelected] = useState<string[]>([]);
  const {
    groups: { getGroups, addUsersToGroups: addUsersToGroup },
  } = useApi();

  const successSubject = useAssignGroupsModal((s) => s.successSubject);
  const closeModal = useAssignGroupsModal((s) => s.close);
  const users = useAssignGroupsModal((s) => s.usersToAssign);
  const queryClient = useQueryClient();
  const toaster = useToaster();
  const { LL } = useI18nContext();

  const { data: groups } = useQuery({
    queryKey: [QueryKeys.FETCH_GROUPS],
    queryFn: async () => getGroups().then((res) => res.groups),
  });

  const { mutate, isPending } = useMutation({
    mutationFn: addUsersToGroup,
    onSuccess: () => {
      invalidateMultipleQueries(queryClient, toInvalidate);
      toaster.success(LL.messages.success());
      successSubject.next();
      closeModal();
    },
  });

  const filteredGroups = useMemo(() => {
    if (groups) {
      if (search.length > 0 && groups) {
        return groups?.filter((g) => g.toLowerCase().includes(search.toLowerCase()));
      }
      return groups ?? [];
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

  const handleSubmit = useCallback(() => {
    if (!isPending) {
      mutate({
        groups: selected,
        users,
      });
    }
  }, [isPending, mutate, selected, users]);

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
            {filteredGroups &&
              filteredGroups?.map((g) => (
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
          loading={isPending}
          onClick={() => handleSubmit()}
        />
      </div>
    </>
  );
};
