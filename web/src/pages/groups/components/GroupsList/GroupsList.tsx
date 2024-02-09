import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { orderBy } from 'lodash-es';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { EditButton } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../shared/defguard-ui/components/Layout/EditButton/types';
import {
  ListHeader,
  ListSortDirection,
} from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { GroupInfo } from '../../../../shared/types';
import { titleCase } from '../../../../shared/utils/titleCase';
import { useAddGroupModal } from '../modals/AddGroupModal/useAddGroupModal';

type Props = {
  groups: GroupInfo[];
  search?: string;
};

type ListData = GroupInfo;

const sortGroups = (groups: GroupInfo[]) => orderBy(groups, ['name'], ['desc']);

export const GroupsList = ({ groups, search }: Props) => {
  const data = useMemo((): ListData[] => {
    if (!search || search.length === 0) {
      return sortGroups(groups);
    }
    return sortGroups(
      groups.filter((g) => g.name.toLowerCase().includes(search.toLowerCase())),
    );
  }, [groups, search]);

  const renderRow = useCallback((data: ListData) => <CustomRow group={data} />, []);

  const listHeaders = useMemo((): ListHeader[] => {
    return [
      {
        key: 'group-name',
        text: 'Group name',
        sortDirection: ListSortDirection.DESC,
        active: true,
      },
      {
        key: 'group-edit',
        text: 'Edit',
        sortable: false,
      },
    ];
  }, []);

  return (
    <VirtualizedList
      id="groups-list"
      headers={listHeaders}
      padding={{
        left: 0,
        right: 0,
      }}
      headerPadding={{
        left: 15,
        right: 15,
      }}
      rowSize={70}
      data={data}
      customRowRender={renderRow}
    />
  );
};

type RowProps = {
  group: GroupInfo;
};

const CustomRow = ({ group }: RowProps) => {
  const openModal = useAddGroupModal((s) => s.open);

  const {
    groups: { deleteGroup },
  } = useApi();

  const toaster = useToaster();
  const { LL } = useI18nContext();
  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation({
    mutationFn: deleteGroup,
    onSuccess: () => {
      toaster.success(LL.messages.success());
      queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_GROUPS_INFO],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_GROUPS],
      });
    },
    onError: () => {
      toaster.error(LL.messages.error());
    },
  });

  return (
    <div className="groups-list-row">
      <div className="group-name left">
        <p>{titleCase(group.name)}</p>
      </div>
      <EditButton>
        <EditButtonOption
          text="Edit"
          onClick={() => {
            openModal(group);
          }}
        />
        {group.name.toLowerCase() !== 'admin' && (
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text="Delete"
            disabled={isLoading}
            onClick={() => {
              mutate(group.name);
            }}
          />
        )}
      </EditButton>
    </div>
  );
};
