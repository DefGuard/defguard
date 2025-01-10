import { useMutation, useQueryClient } from '@tanstack/react-query';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';
import { useDeleteProvisionerModal } from './useDeleteProvisionerModal';

export const DeleteProvisionerModal = () => {
  const isOpen = useDeleteProvisionerModal((state) => state.visible);
  const targetId = useDeleteProvisionerModal((state) => state.provisionerId);
  const [close, reset] = useDeleteProvisionerModal(
    (state) => [state.close, state.reset],
    shallow,
  );

  const { LL } = useI18nContext();
  const {
    provisioning: { deleteWorker },
  } = useApi();
  const toaster = useToaster();

  const queryClient = useQueryClient();

  const { mutate, isPending: isLoading } = useMutation({
    mutationFn: deleteWorker,
    mutationKey: [MutationKeys.DELETE_WORKER],
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_WORKERS],
      });
      toaster.success(
        LL.modals.deleteProvisioner.messages.success({
          provisioner: targetId ?? '',
        }),
      );
      close();
    },
    onError: (e) => {
      toaster.error(LL.messages.error());
      console.error(e);
    },
  });

  return (
    <ConfirmModal
      loading={isLoading}
      submitText="Delete"
      type={ConfirmModalType.WARNING}
      title={LL.modals.deleteProvisioner.title()}
      subTitle={LL.modals.deleteProvisioner.message({ id: targetId ?? '' })}
      isOpen={isOpen}
      onClose={() => close()}
      afterClose={() => reset()}
      onSubmit={() => {
        if (targetId) {
          mutate(targetId);
        }
      }}
    />
  );
};
