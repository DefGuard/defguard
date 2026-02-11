import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../../../../shared/form';
import { formChangeLogic } from '../../../../../../../shared/formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import type { OpenAuthKeyRenameModal } from '../../../../../../../shared/hooks/modalControls/types';

const modalNameKey = ModalName.RenameAuthKey;

export const RenameAuthKeyModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalState, setModalState] = useState<OpenAuthKeyRenameModal | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setModalState(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="rename-auth-key-modal"
      title={m.modal_rename_auth_key_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalState(null);
      }}
    >
      {isPresent(modalState) && <ModalContent {...modalState} />}
    </Modal>
  );
};

const formSchema = z.object({
  name: z.string().trim().min(1, m.form_error_required()),
});

const ModalContent = ({ id, name, username }: OpenAuthKeyRenameModal) => {
  const { mutateAsync } = useMutation({
    mutationFn: api.user.renameAuthKey,
    meta: {
      invalidate: [['user-overview'], ['user', username, 'auth_key']],
    },
    onSuccess: () => {
      closeModal(modalNameKey);
    },
  });

  const form = useAppForm({
    defaultValues: {
      name,
    },
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await mutateAsync({
        id,
        username,
        name: value.name,
      });
    },
  });

  const isPristine = useStore(form.store, (s) => s.isPristine || s.isDefaultValue);
  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  return (
    <>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="name">
            {(field) => <field.FormInput label={m.form_label_name()} required />}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          disabled: form.state.isSubmitting,
          onClick: () => {
            closeModal(modalNameKey);
          },
        }}
        submitProps={{
          text: m.controls_save_changes(),
          loading: isSubmitting,
          disabled: isPristine,
          onClick: () => {
            form.handleSubmit();
          },
        }}
      />
    </>
  );
};
