import { useMutation, useQueryClient } from '@tanstack/react-query';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useDeleteAuthenticationKeyModal } from './useDeleteAuthenticationKeyModal';

export const DeleteAuthenticationKeyModal = () => {
  const {
    user: { deleteAuthenticationKey },
  } = useApi();

  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const [setModalState, authenticationKey, visible] = useDeleteAuthenticationKeyModal(
    (state) => [state.setState, state.authenticationKey, state.visible],
    shallow,
  );

  const toaster = useToaster();

  const { mutate, isLoading } = useMutation(
    (authenticationKeyId: number) => deleteAuthenticationKey(authenticationKeyId),
    {
      onSuccess: () => {
        toaster.success(LL.userPage.authenticationKeys.keyCard.keyDeleted());
        queryClient.invalidateQueries([QueryKeys.FETCH_AUTHENTICATION_KEYS]);
        setModalState({ visible: false, authenticationKey: undefined });
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setModalState({ visible: false, authenticationKey: undefined });
        console.error(err);
      },
    },
  );

  return (
    <ConfirmModal
      isOpen={visible}
      setIsOpen={(v: boolean) => setModalState({ visible: v })}
      type={ConfirmModalType.WARNING}
      title={LL.userPage.authenticationKeys.keyCard.confirmDelete()}
      cancelText={LL.form.cancel()}
      submitText={LL.common.controls.delete()}
      onSubmit={() => {
        if (authenticationKey) {
          mutate(authenticationKey.id);
        }
      }}
      loading={isLoading}
    />
  );
};
