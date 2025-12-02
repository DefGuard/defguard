import z from 'zod';
import { m } from '../../../../../../../paraglide/messages';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { useAppForm } from '../../../../../../../shared/form';
import { formChangeLogic } from '../../../../../../../shared/formLogic';
import {
  closeModal,
  openModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import api from '../../../../../../../shared/api/api';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

const modalNameKey = ModalName.WebauthnSetup;

export const WebautnSetupModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, () => {
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
      id="add-webauthn-modal"
      title={m.modal_mfa_add_passkey_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
    >
      <ModalContent />
    </Modal>
  );
};

const formSchema = z.object({
  name: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .min(
      4,
      m.form_error_min_len({
        length: 4,
      }),
    ),
});

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  name: '',
};

const ModalContent = () => {
  const user = useUserProfile((s) => s.user);
  const queryClient = useQueryClient();

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const { data: backendData } = await api.auth.mfa.webauthn.register.start(
        value.name,
      );
      const creationOpts = PublicKeyCredential.parseCreationOptionsFromJSON(
        backendData.publicKey,
      );
      const credentials = await navigator.credentials.create({ publicKey: creationOpts });
      // if this is null then smth failed or user just canceled the process
      if (credentials != null) {
        const rpkc = (credentials as PublicKeyCredential).toJSON();
        const finishResponse = await api.auth.mfa.webauthn.register.finish({
          name: value.name,
          rpkc: rpkc,
        });
        void queryClient.invalidateQueries({
          queryKey: ['user', user.username],
        });
        const recoveryCodes = finishResponse.data.codes;
        if (isPresent(recoveryCodes)) {
          openModal(ModalName.RecoveryCodes, recoveryCodes);
        }
        closeModal(modalNameKey);
      }
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  return (
    <>
      <p>{m.modal_mfa_add_passkey_content()}</p>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="name">
            {(field) => (
              <field.FormInput label={m.modal_mfa_add_passkey_label()} required />
            )}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          disabled: isSubmitting,
          onClick: () => closeModal(modalNameKey),
        }}
        submitProps={{
          text: m.controls_submit(),
          onClick: () => form.handleSubmit(),
          loading: isSubmitting,
        }}
      />
    </>
  );
};
