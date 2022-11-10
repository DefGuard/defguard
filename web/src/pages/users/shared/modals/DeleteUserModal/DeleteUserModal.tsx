import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import shallow from 'zustand/shallow';

import ConfirmModal, {
  ConfirmModalType,
} from '../../../../../shared/components/layout/ConfirmModal/ConfirmModal';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { User } from '../../../../../shared/types';


const DeleteUserModal = () => {
  const {
    user: { deleteUser },
  } = useApi();

  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const [modalState, setModalState] = useModalStore(
    (state) => [state.deleteUserModal, state.setDeleteUserModal],
    shallow
  );

  const toaster = useToaster();

  const { mutate, isLoading } = useMutation((user: User) => deleteUser(user), {
    onSuccess: (_, variables) => {
      toaster.success(`${variables.username} deleted`);
      queryClient.invalidateQueries([QueryKeys.FETCH_USERS]);
      queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
      setModalState({ visible: false, user: undefined });
      navigate('/admin/users', { replace: true });
    },
    onError: (err) => {
      console.error(err);
      toaster.error('Error occured.');
      setModalState({ visible: false, user: undefined });
    },
  });

  return (
    <ConfirmModal
      isOpen={modalState.visible}
      setIsOpen={(v: boolean) => setModalState({ visible: v })}
      type={ConfirmModalType.WARNING}
      title="Delete account"
      subTitle={`Do you want to delete ${modalState.user?.username} account permanently?`}
      cancelText="Cancel"
      submitText="Delete account"
      onSubmit={() => {
        if (modalState.user) {
          mutate(modalState.user);
        }
      }}
      loading={isLoading}
    />
  );
};

export default DeleteUserModal;
