import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import React, { useEffect, useState } from 'react';

import Badge from '../../../shared/components/layout/Badge/Badge';
import ConfirmModal, {
  ConfirmModalType,
} from '../../../shared/components/layout/ConfirmModal/ConfirmModal';
import { DeviceAvatar } from '../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import IconButton from '../../../shared/components/layout/IconButton/IconButton';
import OptionsPopover from '../../../shared/components/layout/OptionsPopover/OptionsPopover';
import SvgIconCheckmarkGreen from '../../../shared/components/svg/IconCheckmarkGreen';
import SvgIconDisconnected from '../../../shared/components/svg/IconDisconnected';
import SvgIconEditAlt from '../../../shared/components/svg/IconEditAlt';
import SvgIconUserList from '../../../shared/components/svg/IconUserList';
import SvgIconUserListExpanded from '../../../shared/components/svg/IconUserListExpanded';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Provisioner } from '../../../shared/types';
import { tableRowVariants } from '../../../shared/variants';

interface Props {
  provisioners: Provisioner[];
}

const ProvisionersList: React.FC<Props> = ({ provisioners }) => {
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

  return (
    <>
      <ul className="provisioners-list">
        {provisioners.map((provisioner, index) => (
          <ProvisionerListItem
            index={index}
            key={provisioner.id}
            provisioner={provisioner}
            openDelete={openDelete}
          />
        ))}
      </ul>
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

interface ListItemProps {
  provisioner: Provisioner;
  openDelete: (id: string) => void;
  index: number;
}

const ProvisionerListItem: React.FC<ListItemProps> = ({
  provisioner,
  openDelete,
  index,
}) => {
  const [open, setOpen] = useState(false);
  const [editElement, setEditElement] = useState<HTMLButtonElement | null>(
    null
  );
  const [editOpen, setEditOpen] = useState(false);

  return (
    <motion.li
      custom={index}
      variants={tableRowVariants}
      initial="hidden"
      animate="idle"
    >
      <div className={open ? 'provisioner open' : 'provisioner'}>
        <div className="top">
          <div className="expand" onClick={() => setOpen((state) => !state)}>
            {open ? <SvgIconUserListExpanded /> : <SvgIconUserList />}
            <DeviceAvatar active={provisioner.connected} />
          </div>
          <div className="info">
            <p className="id">{provisioner.id}</p>
            <div className="badges">
              <Badge text={provisioner.ip} />
            </div>
          </div>
          <IconButton
            className="blank"
            ref={setEditElement}
            onClick={() => setEditOpen(true)}
          >
            <SvgIconEditAlt />
          </IconButton>
          {editElement ? (
            <OptionsPopover
              referenceElement={editElement}
              isOpen={editOpen}
              setIsOpen={setEditOpen}
              popperOptions={{ position: 'left' }}
              items={[
                <button
                  key="delete-provisioner"
                  className="warning"
                  onClick={() => {
                    openDelete(provisioner.id);
                    setEditOpen(false);
                  }}
                >
                  Delete
                </button>,
              ]}
            />
          ) : null}
        </div>
        {open ? <div className="divider"></div> : null}
        {open ? (
          <div className="collapsible">
            <div className="labeled-group">
              <label>Status:</label>
              <div className="status">
                {provisioner.connected ? (
                  <SvgIconCheckmarkGreen />
                ) : (
                  <SvgIconDisconnected />
                )}
                <span>
                  {provisioner.connected ? 'Available' : 'Unavailable'}
                </span>
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </motion.li>
  );
};

export default ProvisionersList;
