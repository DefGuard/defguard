import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import classNames from 'classnames';
import { useEffect, useMemo, useState } from 'react';

import { AvatarBox } from '../../../shared/components/layout/AvatarBox/AvatarBox';
import ConfirmModal, {
  ConfirmModalType,
} from '../../../shared/components/layout/ConfirmModal/ConfirmModal';
import { DeviceAvatar } from '../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../shared/components/layout/EditButton/EditButtonOption';
import {
  ListHeader,
  ListSortDirection,
  VirtualizedList,
} from '../../../shared/components/layout/VirtualizedList/VirtualizedList';
import {
  IconCheckmarkGreen,
  IconDeactivated,
} from '../../../shared/components/svg';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Provisioner, SelectOption } from '../../../shared/types';

interface Props {
  provisioners: Provisioner[];
}

export const ProvisionersList = ({ provisioners }: Props) => {
  const {
    provisioning: { deleteWorker },
  } = useApi();

  const queryClient = useQueryClient();

  const { mutate: deleteWorkerMutate } = useMutation(deleteWorker, {
    mutationKey: [MutationKeys.DELETE_WORKER],
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_WORKERS]);
    },
  });

  const [deleteModalOpen, setDeleteModalOpen] = useState(false);

  const [deleteTarget, setDeleteTarget] = useState('');

  const openDelete = (id: string) => {
    setDeleteModalOpen(true);
    setDeleteTarget(id);
  };

  const onDelete = () => {
    if (deleteTarget.length) {
      deleteWorkerMutate(deleteTarget);
    }
  };

  useEffect(() => {
    if (!deleteModalOpen) {
      setDeleteTarget('');
    }
  }, [deleteModalOpen]);

  const listCells = useMemo(
    () => [
      {
        key: 'name',
        render: (value: Provisioner) => (
          <>
            <span className={classNames({ connected: value.connected })}>
              {value.id}
            </span>
          </>
        ),
      },
      {
        key: 'status',
        render: (value: Provisioner) => (
          <>
            {value.connected ? (
              <>
                <IconCheckmarkGreen />
                <span className={classNames({ connected: value.connected })}>
                  Available
                </span>
              </>
            ) : (
              <>
                <IconDeactivated />
                <span className={classNames({ connected: value.connected })}>
                  Unavailable
                </span>
              </>
            )}
          </>
        ),
      },
      {
        key: 'ip',
        render: (value: Provisioner) => (
          <span className={classNames({ connected: value.connected })}>
            {value.ip}
          </span>
        ),
      },
      {
        key: 'edit',
        render: (value: Provisioner) => (
          <EditButton>
            <EditButtonOption
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              onClick={() => openDelete(value.id)}
              text="Remove provisioner"
            />
          </EditButton>
        ),
      },
    ],
    []
  );

  const getListHeaders = useMemo(() => {
    const res: ListHeader[] = [
      {
        key: 'name',
        text: 'Name',
        active: true,
        sortDirection: ListSortDirection.ASC,
      },
      {
        key: 'status',
        text: 'Status',
        active: false,
      },
      {
        key: 'ip',
        text: 'IP address',
        active: false,
      },
      {
        key: 'actions',
        text: 'Actions',
        active: false,
      },
    ];
    return res;
  }, []);

  return (
    <>
      <VirtualizedList
        className="provisioners-list"
        rowSize={70}
        data={provisioners}
        headers={getListHeaders}
        cells={listCells}
        padding={{
          left: 60,
          right: 40,
        }}
        headerPadding={{
          right: 25,
          left: 20,
        }}
      />
      <ConfirmModal
        isOpen={deleteModalOpen}
        setIsOpen={setDeleteModalOpen}
        onSubmit={onDelete}
        submitText="Delete"
        title={`Delete provisioner`}
        subTitle={`Provisioner ${deleteTarget} will be deleted`}
        type={ConfirmModalType.WARNING}
      />
    </>
  );
};
