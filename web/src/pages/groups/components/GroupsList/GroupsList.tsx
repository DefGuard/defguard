import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { orderBy } from 'lodash-es';
import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { EditButton } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { ConfirmModal } from '../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import {
  ListHeader,
  ListSortDirection,
} from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { GroupInfo } from '../../../../shared/types';
import { invalidateMultipleQueries } from '../../../../shared/utils/invalidateMultipleQueries';
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

  const adminGroupCount = useCallback(() => {
    return groups.filter((group) => group.is_admin).length;
  }, [groups]);

  const renderRow = useCallback(
    (data: ListData) => (
      <CustomRow group={data} disableDelete={adminGroupCount() === 1 && data.is_admin} />
    ),
    [adminGroupCount],
  );

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
  disableDelete: boolean;
};

const CustomRow = ({ group, disableDelete }: RowProps) => {
  const openModal = useAddGroupModal((s) => s.open);
  const [isDeleteModalOpen, setDeleteModalOpen] = useState(false);

  const {
    groups: { deleteGroup },
  } = useApi();

  const toaster = useToaster();
  const { LL } = useI18nContext();
  const queryClient = useQueryClient();

  const { mutate, isPending: isLoading } = useMutation({
    mutationFn: deleteGroup,
    onSuccess: () => {
      toaster.success(LL.messages.success());
      invalidateMultipleQueries(queryClient, [
        QueryKeys.FETCH_GROUPS,
        QueryKeys.FETCH_GROUPS_INFO,
      ]);
      setDeleteModalOpen(false);
    },
    onError: () => {
      toaster.error(LL.messages.error());
    },
  });

  const locationList = (locations: string[]) => (
    <>
      <br />
      <div>
        <p>{LL.modals.deleteGroup.locationListHeader()}</p>
        <ul>
          {locations.map((locationName) => (
            <li key={locationName}>{locationName}</li>
          ))}
        </ul>
        <br />
        <br />
        <p>{parse(LL.modals.deleteGroup.locationListFooter())}</p>
      </div>
    </>
  );

  return (
    <>
      <div className="groups-list-row">
        <div className="group-name left">
          <p>{titleCase(group.name)}</p>
        </div>
        <EditButton>
          <EditButtonOption
            text={LL.common.controls.edit()}
            onClick={() => {
              openModal(group);
            }}
          />
          {!disableDelete && (
            <EditButtonOption
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              text={LL.common.controls.delete()}
              disabled={isLoading}
              onClick={() => {
                setDeleteModalOpen(true);
              }}
            />
          )}
        </EditButton>
      </div>
      <ConfirmModal
        id="group-delete-modal"
        type={ConfirmModalType.WARNING}
        isOpen={isDeleteModalOpen}
        setIsOpen={(v) => setDeleteModalOpen(v)}
        onSubmit={() => mutate(group.name)}
        onCancel={() => setDeleteModalOpen(false)}
        title={LL.modals.deleteGroup.title({
          name: group.name,
        })}
        subTitle={
          <div>
            <p>{LL.modals.deleteGroup.subTitle()}</p>
            {group.vpn_locations.length > 0 && locationList(group.vpn_locations)}
          </div>
        }
        submitText={LL.modals.deleteGroup.submit()}
        cancelText={LL.modals.deleteGroup.cancel()}
        loading={isLoading}
      />
    </>
  );
};
