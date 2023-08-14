import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import classNames from 'classnames';
import { useEffect, useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../i18n/i18n-react';
import IconCheckmarkGreen from '../../../shared/components/svg/IconCheckmarkGreen';
import IconDeactivated from '../../../shared/components/svg/IconDeactivated';
import { deviceBreakpoints } from '../../../shared/constants';
import { EditButton } from '../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../shared/defguard-ui/components/Layout/EditButton/types';
import ConfirmModal from '../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import {
  ListHeader,
  ListSortDirection,
} from '../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Provisioner } from '../../../shared/types';

interface Props {
  provisioners: Provisioner[];
}

export const ProvisionersList = ({ provisioners }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const { LL } = useI18nContext();
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

  const listCells = useMemo(() => {
    const res = [
      {
        key: 'name',
        render: (value: Provisioner) => (
          <>
            <span className={classNames({ connected: value.connected })}>{value.id}</span>
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
                  {LL.provisionersOverview.list.status.available()}
                </span>
              </>
            ) : (
              <>
                <IconDeactivated />
                <span className={classNames({ connected: value.connected })}>
                  {LL.provisionersOverview.list.status.unavailable()}
                </span>
              </>
            )}
          </>
        ),
      },
      {
        key: 'ip',
        render: (value: Provisioner) => (
          <span className={classNames({ connected: value.connected })}>{value.ip}</span>
        ),
      },
      {
        key: 'edit',
        render: (value: Provisioner) => (
          <EditButton>
            <EditButtonOption
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              onClick={() => openDelete(value.id)}
              text={LL.provisionersOverview.list.editButton.delete()}
            />
          </EditButton>
        ),
      },
    ];
    if (breakpoint !== 'desktop') {
      res.splice(1, 1);
    }
    return res;
  }, [
    LL.provisionersOverview.list.editButton,
    LL.provisionersOverview.list.status,
    breakpoint,
  ]);

  const getListHeaders = useMemo(() => {
    const res: ListHeader[] = [
      {
        key: 'name',
        text: LL.provisionersOverview.list.headers.name(),
        active: true,
        sortDirection: ListSortDirection.ASC,
      },
      {
        key: 'status',
        text: LL.provisionersOverview.list.headers.status(),
        active: false,
      },
      {
        key: 'ip',
        text: LL.provisionersOverview.list.headers.ip(),
        active: false,
      },
      {
        key: 'actions',
        text: LL.provisionersOverview.list.headers.actions(),
        active: false,
        sortable: false,
      },
    ];
    if (breakpoint !== 'desktop') {
      res.splice(1, 1);
    }
    return res;
  }, [LL.provisionersOverview.list.headers, breakpoint]);

  return (
    <>
      <VirtualizedList
        className="provisioners-list"
        rowSize={70}
        data={provisioners}
        headers={getListHeaders}
        cells={listCells}
        padding={{
          left: breakpoint === 'desktop' ? 60 : 20,
          right: 20,
        }}
        headerPadding={{
          right: 20,
          left: 20,
        }}
      />
      <ConfirmModal
        isOpen={deleteModalOpen}
        setIsOpen={setDeleteModalOpen}
        onSubmit={onDelete}
        submitText="Delete"
        title={LL.modals.deleteProvisioner.title()}
        subTitle={LL.modals.deleteProvisioner.message({ id: deleteTarget })}
        type={ConfirmModalType.WARNING}
      />
    </>
  );
};
