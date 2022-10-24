import { useMutation, useQueryClient } from '@tanstack/react-query';
import React from 'react';
import { toast } from 'react-toastify';
import shallow from 'zustand/shallow';

import ConfirmModal, {
  ConfirmModalType,
} from '../../../../shared/components/layout/ConfirmModal/ConfirmModal';
import ToastContent, {
  ToastType,
} from '../../../../shared/components/Toasts/ToastContent';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../shared/queries';
import { OpenidClient } from '../../../../shared/types';

const EnableClientModal: React.FC = () => {
  const {
    openid: { changeOpenidClientState },
  } = useApi();

  const queryClient = useQueryClient();

  const [modalState, setModalState] = useModalStore(
    (state) => [
      state.enableOpenidClientModal,
      state.setEnableOpenidClientModal,
    ],
    shallow
  );

  const { mutate, isLoading } = useMutation(
    (client: OpenidClient) =>
      changeOpenidClientState({ id: client.id, enabled: !client.enabled }),
    {
      onSuccess: (_, variables) => {
        toast(
          <ToastContent
            type={ToastType.SUCCESS}
            message={`${variables.name} `}
          />
        );
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
        if (modalState.onSuccess) {
          modalState.onSuccess();
        }
        setModalState({ visible: false });
      },
      onError: () => {
        setModalState({ visible: false });
      },
    }
  );

  return (
    <ConfirmModal
      isOpen={modalState.visible}
      setIsOpen={(v: boolean) => setModalState({ visible: v })}
      type={
        modalState.client?.enabled
          ? ConfirmModalType.WARNING
          : ConfirmModalType.NORMAL
      }
      title={modalState.client?.enabled ? 'Disable app' : 'Enable app'}
      subTitle={`Do you want to ${
        modalState.client?.enabled ? 'disable' : 'enable'
      } ${modalState.client?.name} app?`}
      cancelText="Cancel"
      submitText={modalState.client?.enabled ? 'Disable ' : 'Enable '}
      onSubmit={() => {
        if (modalState.client) {
          mutate(modalState.client);
        }
      }}
      loading={isLoading}
    />
  );
};

export default EnableClientModal;
