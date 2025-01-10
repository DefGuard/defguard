import { useMutation, useQueryClient } from '@tanstack/react-query';
import { cloneDeep, isUndefined } from 'lodash-es';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { invalidateMultipleQueries } from '../../../../../shared/utils/invalidateMultipleQueries';

export const ToggleUserModal = () => {
  const {
    user: { editUser },
  } = useApi();

  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const [modalState, setModalState] = useModalStore(
    (state) => [state.toggleUserModal, state.setToggleUserModal],
    shallow,
  );

  const toaster = useToaster();

  const { mutate, isPending } = useMutation({
    mutationFn: editUser,
    onSuccess: (_, variables) => {
      toaster.success(
        variables.data.is_active
          ? LL.modals.enableUser.messages.success({
              username: variables.username,
            })
          : LL.modals.disableUser.messages.success({
              username: variables.username,
            }),
      );
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_USER_PROFILE],
        [QueryKeys.FETCH_USERS_LIST],
      ]);
      setModalState({ visible: false, user: undefined });
      navigate('/admin/users', { replace: true });
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      setModalState({ visible: false, user: undefined });
      console.error(err);
    },
  });

  const toggleUserState = () => {
    if (!isUndefined(modalState.user)) {
      const userClone = cloneDeep(modalState.user);
      userClone.is_active = !userClone?.is_active;
      mutate({
        username: userClone.username,
        data: userClone,
      });
    }
  };

  return (
    <ConfirmModal
      isOpen={modalState.visible}
      setIsOpen={(v: boolean) => setModalState({ visible: v })}
      type={
        modalState.user?.is_active ? ConfirmModalType.WARNING : ConfirmModalType.NORMAL
      }
      subTitle={
        modalState.user?.is_active
          ? LL.modals.disableUser.message({
              username: modalState.user?.username || '',
            })
          : LL.modals.enableUser.message({
              username: modalState.user?.username || '',
            })
      }
      submitText={
        modalState.user?.is_active
          ? LL.modals.disableUser.controls.submit()
          : LL.modals.enableUser.controls.submit()
      }
      title={
        modalState.user?.is_active
          ? LL.modals.disableUser.title()
          : LL.modals.enableUser.title()
      }
      cancelText={LL.form.cancel()}
      onSubmit={() => {
        toggleUserState();
      }}
      loading={isPending}
    />
  );
};
