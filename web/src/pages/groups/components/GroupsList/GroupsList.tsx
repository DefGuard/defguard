import './style.scss';

import { useCallback, useMemo } from 'react';

import { EditButton } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../shared/defguard-ui/components/Layout/EditButton/types';
import {
  ListHeader,
  ListSortDirection,
} from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import { GroupInfo } from '../../../../shared/types';
import { titleCase } from '../../../../shared/utils/titleCase';
import { useAddGroupModal } from '../modals/AddGroupModal/useAddGroupModal';

type Props = {
  groups: GroupInfo[];
  search?: string;
};

type ListData = GroupInfo;

export const GroupsList = ({ groups, search }: Props) => {
  const data = useMemo((): ListData[] => {
    if (!search || search.length === 0) {
      return groups;
    }
    return groups.filter((g) => g.name.toLowerCase().includes(search.toLowerCase()));
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
  return (
    <div className="groups-list-row">
      <div className="group-name left">
        <p>{titleCase(group.name)}</p>
      </div>
      <EditButton>
        {group.name.toLowerCase() !== 'admin' && (
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text="Delete"
            onClick={() => {
              return;
            }}
          />
        )}
        <EditButtonOption
          text="Edit"
          onClick={() => {
            openModal(group);
          }}
        />
      </EditButton>
    </div>
  );
};
