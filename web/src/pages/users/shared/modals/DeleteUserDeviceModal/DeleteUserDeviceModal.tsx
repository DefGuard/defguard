import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';

import ConfirmModal, {
  ConfirmModalType,
} from '../../../../../shared/components/layout/ConfirmModal/ConfirmModal';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';

export const DeleteUserDeviceModal = () => {
  const toaster = useToaster();
  const modalState = useModalStore((state) => state.deleteUserDeviceModal);
  const setModalState = useModalStore(
    (state) => state.setDeleteUserDeviceModal
  );
  const {
    device: { deleteDevice },
  } = useApi();
  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation(
    [MutationKeys.DELETE_USER_DEVICE],
    deleteDevice,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        setModalState({ visible: false, device: undefined });
        toaster.success('Device deleted.');
      },
      onError: (err: AxiosError) => {
        console.error(err);
        toaster.error('Error ocurred. Please contact with administrator.');
      },
    }
  );

  return (
    <ConfirmModal
      title="Delete device"
      type={ConfirmModalType.WARNING}
      subTitle={`Do you want to delete ${modalState.device?.name} device ?`}
      cancelText="Cancel"
      submitText="Delete device"
      loading={isLoading || !modalState.device}
      isOpen={modalState.visible}
      setIsOpen={(visibility) => setModalState({ visible: visibility })}
      onSubmit={() => {
        if (modalState.device) {
          mutate(modalState.device);
        }
      }}
    />
  );
};
