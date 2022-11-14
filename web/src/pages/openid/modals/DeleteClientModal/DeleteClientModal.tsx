import { useMutation, useQueryClient } from '@tanstack/react-query';
import React from 'react';
import shallow from 'zustand/shallow';

import ConfirmModal, {
  ConfirmModalType,
} from '../../../../shared/components/layout/ConfirmModal/ConfirmModal';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../shared/queries';
import { OpenidClient } from '../../../../shared/types';

const DeleteClientModal: React.FC = () => {
  const {
    openid: { deleteOpenidClient },
  } = useApi();

  const queryClient = useQueryClient();

  const [modalState, setModalState] = useModalStore(
    (state) => [
      state.deleteOpenidClientModal,
      state.setDeleteOpenidClientModal,
    ],
    shallow
  );

  const { mutate, isLoading } = useMutation(
    (client: OpenidClient) => deleteOpenidClient(client.id),
    {
      onSuccess: () => {
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
      type={ConfirmModalType.WARNING}
      title="Delete app"
      subTitle={`Do you want to delete ${modalState.client?.name} app permanently?`}
      cancelText="Cancel"
      submitText="Delete app"
      onSubmit={() => {
        if (modalState.client) {
          mutate(modalState.client);
        }
      }}
      loading={isLoading}
    />
  );
};

export default DeleteClientModal;
