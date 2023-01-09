import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import ConfirmModal, {
  ConfirmModalType,
} from '../../../../../../shared/components/layout/ConfirmModal/ConfirmModal';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';

export const DeleteUserDeviceModal = () => {
  const { LL } = useI18nContext();
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
        toaster.success(LL.modals.deleteDevice.messages.success());
      },
      onError: (err: AxiosError) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );

  return (
    <ConfirmModal
      title={LL.modals.deleteDevice.title()}
      type={ConfirmModalType.WARNING}
      subTitle={LL.modals.deleteDevice.message({
        deviceName: modalState.device?.name || '',
      })}
      cancelText={LL.form.cancel()}
      submitText={LL.modals.deleteDevice.submit()}
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
