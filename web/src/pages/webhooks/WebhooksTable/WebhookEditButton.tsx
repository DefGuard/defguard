import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'framer-motion';
import React, { useState } from 'react';
import ClickAwayListener from 'react-click-away-listener';
import { toast } from 'react-toastify';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import IconButton from '../../../shared/components/layout/IconButton/IconButton';
import OptionsPopover from '../../../shared/components/layout/OptionsPopover/OptionsPopover';
import SvgIconEditAlt from '../../../shared/components/svg/IconEditAlt';
import ToastContent, {
  ToastType,
} from '../../../shared/components/layout/Toast/Toast';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Webhook } from '../../../shared/types';
import { standardVariants } from '../../../shared/variants';
import EditWebhookModal from '../modals/EditWebhookModal/EditWebhookModal';

type EditButtonProps = {
  id: string;
  enabled: boolean;
  webhook: Webhook;
};

const WebhookEditButton: React.FC<EditButtonProps> = ({
  id,
  enabled,
  webhook,
}) => {
  const [refElement, setRefElement] = useState<HTMLButtonElement | null>();
  const [optionsVisible, setOptionsVisible] = useState(false);
  const [isDeleteOpen, setDeleteOpen] = useState(false);
  const [isWebhookStateOpen, setWebhookStateOpen] = useState(false);

  const setEditWebhookModalState = useModalStore(
    (state) => state.setEditWebhookModal
  );

  const {
    webhook: { deleteWebhook, changeWebhookState },
  } = useApi();

  const queryClient = useQueryClient();
  const deleteWebhookMutation = useMutation((id: string) => deleteWebhook(id), {
    onSuccess: () => {
  
      setDeleteOpen(false);
      queryClient.invalidateQueries([QueryKeys.FETCH_WEBHOOKS]);
    },
    onError: () => {
      setDeleteOpen(false);
    },
  });

  const changeWebhookStateMutation = useMutation(changeWebhookState, {
    mutationKey: [MutationKeys.CHANGE_WEBHOOK_STATE],
    onSuccess: () => {
      setWebhookStateOpen(false);
  
      queryClient.invalidateQueries([QueryKeys.FETCH_WEBHOOKS]);
    },
    onError: () => {
      setWebhookStateOpen(false);

    },
  });
  return (
    <>
      <td className="edit" role="cell">
        <IconButton className="blank" ref={setRefElement}>
          <SvgIconEditAlt />
        </IconButton>
      </td>
      {refElement ? (
        <OptionsPopover
          referenceElement={refElement}
          isOpen={optionsVisible}
          setIsOpen={setOptionsVisible}
          items={[
            <button
              key="delete"
              className="warning"
              onClick={() => {
                setOptionsVisible(false);
                setDeleteOpen(true);
              }}
            >
              Delete
            </button>,
            <button
              key="change-state"
              onClick={() => {
                setOptionsVisible(false);
                setWebhookStateOpen(true);
              }}
            >
              {enabled ? 'Disable' : 'Enable'}
            </button>,
            <button
              key="edit-webhook"
              onClick={() => {
                setEditWebhookModalState({ visible: true });
              }}
            >
              Edit
            </button>,
          ]}
        />
      ) : null}
      <AnimatePresence mode="wait">
        {isDeleteOpen ? (
          <ClickAwayListener onClickAway={() => setDeleteOpen(false)}>
            <motion.div
              className="row-overlay delete"
              initial="hidden"
              animate="show"
              exit="hidden"
              variants={standardVariants}
            >
              <Button
                size={ButtonSize.SMALL}
                styleVariant={ButtonStyleVariant.STANDARD}
                onClick={() => setDeleteOpen(false)}
                text="Cancel"
              />
              <Button
                styleVariant={ButtonStyleVariant.CONFIRM_WARNING}
                size={ButtonSize.SMALL}
                onClick={() => deleteWebhookMutation.mutate(id)}
                text="Delete"
              />
            </motion.div>
          </ClickAwayListener>
        ) : null}
        {isWebhookStateOpen ? (
          <ClickAwayListener onClickAway={() => setWebhookStateOpen(false)}>
            <motion.div
              className="row-overlay delete"
              initial="hidden"
              animate="show"
              exit="hidden"
              variants={standardVariants}
            >
              <Button
                size={ButtonSize.SMALL}
                styleVariant={ButtonStyleVariant.STANDARD}
                onClick={() => setWebhookStateOpen(false)}
                text="Cancel"
              />
              <Button
                styleVariant={
                  enabled
                    ? ButtonStyleVariant.WARNING
                    : ButtonStyleVariant.PRIMARY
                }
                size={ButtonSize.SMALL}
                onClick={() =>
                  changeWebhookStateMutation.mutate({
                    id: id,
                    enabled: enabled ? false : true,
                  })
                }
                text={enabled ? 'Disable' : 'Enable'}
              />
            </motion.div>
          </ClickAwayListener>
        ) : null}
      </AnimatePresence>
      <EditWebhookModal webhook={webhook} />
    </>
  );
};

export default WebhookEditButton;
