import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import React, { useEffect, useMemo, useState } from 'react';

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
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { patternBaseUrl } from '../../../shared/patterns';
import { QueryKeys } from '../../../shared/queries';
import { Webhook } from '../../../shared/types';
import { tableRowVariants } from '../../../shared/variants';
import EditWebhookModal from '../modals/EditWebhookModal/EditWebhookModal';

interface Props {
  webhooks: Webhook[];
}

const WebhooksList: React.FC<Props> = ({ webhooks }) => {
  const {
    webhook: { deleteWebhook, changeWebhookState },
  } = useApi();

  const queryClient = useQueryClient();

  const { mutate: deleteWebhookMutate } = useMutation(deleteWebhook, {
    mutationKey: [MutationKeys.DELETE_WEBHOOK],
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_WEBHOOKS]);
    },
  });

  const [deleteModalOpen, setDeleteModalOpen] = useState(false);

  const [deleteTarget, setDeleteTarget] = useState('');
  const [enableTarget, setEnableTarget] = useState({ id: '', enabled: false });

  const openDelete = (id: string) => {
    setDeleteModalOpen(true);
    setDeleteTarget(id);
  };

  const onDelete = () => {
    if (deleteTarget.length) {
      deleteWebhookMutate(deleteTarget);
    }
  };

  useEffect(() => {
    if (!deleteModalOpen) {
      setDeleteTarget('');
    }
  }, [deleteModalOpen]);

  const [enableModalOpen, setEnableModalOpen] = useState(false);
  const changeWebhookStateMutation = useMutation(changeWebhookState, {
    mutationKey: [MutationKeys.CHANGE_WEBHOOK_STATE],
    onSuccess: () => {
      setEnableModalOpen(false);
      queryClient.invalidateQueries([QueryKeys.FETCH_WEBHOOKS]);
    },
    onError: () => {
      setEnableModalOpen(false);
    },
  });
  const openEnable = (webhook: Webhook) => {
    setEnableModalOpen(true);
    setEnableTarget({ id: webhook.id, enabled: webhook.enabled });
  };

  const onEnable = () => {
    if (enableTarget !== null) {
      changeWebhookStateMutation.mutate({
        id: enableTarget.id,
        enabled: enableTarget.enabled ? false : true,
      });
    }
  };

  return (
    <>
      <ul className="webhooks-list">
        {webhooks.map((webhook, index) => (
          <WebhooksListItem
            index={index}
            key={webhook.id}
            webhook={webhook}
            openDelete={openDelete}
            openEnable={openEnable}
          />
        ))}
      </ul>
      <ConfirmModal
        isOpen={deleteModalOpen}
        setIsOpen={setDeleteModalOpen}
        onSubmit={onDelete}
        submitText="Delete"
        title={`Delete webhook`}
        subTitle={`Webhook ${deleteTarget} will be deleted`}
        type={ConfirmModalType.WARNING}
      />
      <ConfirmModal
        isOpen={enableModalOpen}
        setIsOpen={setEnableModalOpen}
        onSubmit={onEnable}
        submitText={enableTarget.enabled ? 'Disable' : 'Enable'}
        title={enableTarget.enabled ? 'Disable Webhook' : 'Enable Webhook'}
        subTitle={
          enableTarget.enabled
            ? 'Webhook will be disabled'
            : 'Webhook will be enabled'
        }
        type={
          enableTarget.enabled
            ? ConfirmModalType.WARNING
            : ConfirmModalType.NORMAL
        }
      />
    </>
  );
};

interface ListItemProps {
  webhook: Webhook;
  openDelete: (id: string) => void;
  openEnable: (webhook: Webhook) => void;
  index: number;
}

const WebhooksListItem: React.FC<ListItemProps> = ({
  webhook,
  openDelete,
  openEnable,
  index,
}) => {
  const [open, setOpen] = useState(false);
  const [editElement, setEditElement] = useState<HTMLButtonElement | null>(
    null
  );
  const [editOpen, setEditOpen] = useState(false);
  const setEditWebhookModalState = useModalStore(
    (state) => state.setEditWebhookModal
  );

  const getId = useMemo(() => {
    const match = webhook.url.match(patternBaseUrl);
    if (match) {
      return match[1];
    }
  }, [webhook.url]);

  return (
    <motion.li
      custom={index}
      variants={tableRowVariants}
      initial="hidden"
      animate="idle"
    >
      <div className={open ? 'webhook open' : 'webhook'}>
        <div className="top">
          <div className="expand" onClick={() => setOpen((state) => !state)}>
            {open ? <SvgIconUserListExpanded /> : <SvgIconUserList />}
            <DeviceAvatar active={webhook.enabled} />
          </div>
          <div className="info">
            <p className="id">{getId}</p>
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
                  key="delete-webhook"
                  className="warning"
                  onClick={() => {
                    openDelete(webhook.id);
                    setEditOpen(false);
                  }}
                >
                  Delete
                </button>,
                <button
                  key="state-webhook"
                  onClick={() => {
                    openEnable(webhook);
                  }}
                >
                  {webhook.enabled ? 'Disable' : 'Enable'}
                </button>,
                <button
                  key="edit-webhook"
                  onClick={() => {
                    setEditWebhookModalState({ visible: true });
                  }}
                >
                  Edit webhook
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
                {webhook.enabled ? (
                  <SvgIconCheckmarkGreen />
                ) : (
                  <SvgIconDisconnected />
                )}
                <span>{webhook.enabled ? 'Enabled' : 'Disabled'}</span>
              </div>
              <label>Description:</label>
              <div className="description">
                <span>{webhook.description}</span>
              </div>
            </div>
          </div>
        ) : null}
      </div>
      <EditWebhookModal webhook={webhook} />
    </motion.li>
  );
};

export default WebhooksList;
