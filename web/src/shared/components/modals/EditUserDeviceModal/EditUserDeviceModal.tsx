import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { cloneDeep } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import api from '../../../api/api';
import { Modal } from '../../../defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../defguard-ui/types';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import { useAppForm } from '../../../form';
import { formChangeLogic } from '../../../formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../hooks/modalControls/modalTypes';
import type { OpenEditDeviceModal } from '../../../hooks/modalControls/types';
import { patternValidWireguardKey } from '../../../patterns';

const modalName = ModalName.EditUserDevice;

export const EditUserDeviceModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<OpenEditDeviceModal | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalName, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalName, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      title={m.modal_edit_user_device_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent {...modalData} />}
    </Modal>
  );
};

const getFormSchema = (names: string[]) =>
  z.object({
    name: z
      .string()
      .trim()
      .min(1, m.form_error_required())
      .refine((val) => !names.includes(val), m.form_error_name_reserved()),
    publicKey: z
      .string()
      .length(44, m.form_error_invalid())
      .regex(patternValidWireguardKey, m.form_error_invalid()),
  });

const ModalContent = ({ device, reservedNames, username }: OpenEditDeviceModal) => {
  const formSchema = useMemo(
    () => getFormSchema(reservedNames.filter((name) => name !== device.name)),
    [reservedNames, device.name],
  );

  const { mutateAsync } = useMutation({
    mutationFn: api.device.editDevice,
    meta: {
      invalidate: [['user-overview'], ['user', username]],
    },
    onSuccess: () => {
      closeModal(modalName);
    },
  });

  const form = useAppForm({
    defaultValues: {
      name: device.name,
      publicKey: device.wireguard_pubkey,
    },
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const copy = cloneDeep(device);
      copy.name = value.name;
      copy.wireguard_pubkey = value.publicKey;
      mutateAsync(copy);
    },
  });

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
            {(field) => <field.FormInput label={m.form_label_device_name()} required />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="publicKey">
            {(field) => (
              <field.FormInput label={m.form_label_device_public_key()} required />
            )}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          disabled: isSubmitting,
          onClick: () => {
            closeModal(modalName);
          },
        }}
        submitProps={{
          text: m.controls_save_changes(),
          loading: isSubmitting,
          onClick: () => {
            form.handleSubmit();
          },
        }}
      />
    </>
  );
};
