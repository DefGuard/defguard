import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { useAppForm } from '../../../../../../../shared/defguard-ui/form';
import { formChangeLogic } from '../../../../../../../shared/form';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import api from '../../../../../../../shared/api/api';
import {
  AuthKeyType,
  type AuthKeyTypeValue,
} from '../../../../../../../shared/api/types';
import { Select } from '../../../../../../../shared/defguard-ui/components/Select/Select';
import type { SelectOption } from '../../../../../../../shared/defguard-ui/components/Select/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';

const modalNameKey = ModalName.AddAuthKey;

export const AddAuthKeyModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [username, setUsername] = useState<string | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setUsername(data.username);
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
      id="add-auth-key-modal"
      title={m.modal_add_auth_key_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setUsername(null);
      }}
    >
      {isPresent(username) && <ModalContent username={username} />}
    </Modal>
  );
};

const selectOptions: SelectOption<AuthKeyTypeValue>[] = [
  {
    key: AuthKeyType.SSH,
    value: AuthKeyType.SSH,
    label: 'SSH',
  },
  {
    key: AuthKeyType.GPG,
    value: AuthKeyType.GPG,
    label: 'GPG',
  },
] as const;

const getFormSchema = () =>
  z.object({
    name: z.string().trim().min(1, m.form_error_required()),
    key: z.string().trim().min(1, m.form_error_required()),
  });

type FormFields = z.infer<ReturnType<typeof getFormSchema>>;

const defaultValues: FormFields = {
  key: '',
  name: '',
};

const ModalContent = ({ username }: { username: string }) => {
  const [selected, setSelected] = useState(selectOptions[0]);
  const formSchema = useMemo(() => getFormSchema(), []);

  const { mutateAsync: addKey } = useMutation({
    mutationFn: api.user.addAuthKey,
    meta: {
      invalidate: ['user', username],
    },
    onSuccess: () => {
      closeModal(modalNameKey);
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await addKey({
        ...value,
        key_type: selected.value,
        username,
      });
    },
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect
  useEffect(() => {
    if (!form.state.isPristine) {
      form.validateAllFields('change');
    }
  }, [selected]);

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
          <Select
            value={selected}
            onChange={setSelected}
            options={selectOptions}
            testId="field-type"
            label={m.form_label_type()}
          />
          <form.AppField name="name">
            {(field) => <field.FormInput label={m.form_label_name()} required />}
          </form.AppField>
          <form.AppField name="key">
            {(field) => <field.FormInput label={m.form_label_key()} required />}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          onClick: () => {
            closeModal(modalNameKey);
          },
        }}
        submitProps={{
          text: m.modal_add_auth_key_submit(),
          onClick: () => {
            form.handleSubmit();
          },
        }}
      />
    </>
  );
};
