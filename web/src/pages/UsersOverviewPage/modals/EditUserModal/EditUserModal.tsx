import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { cloneDeep } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { EvenSplit } from '../../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenEditUserModal } from '../../../../shared/hooks/modalControls/types';
import { patternSafeUsernameCharacters } from '../../../../shared/patterns';
import { removeEmptyStrings } from '../../../../shared/utils/removeEmptyStrings';

const modalName = ModalName.EditUserModal;

type ModalData = OpenEditUserModal;

export const EditUserModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

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
      id="edit-user-modal"
      title={m.modal_edit_user_title()}
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

const ModalContent = ({ reservedEmails, reservedUsernames, user }: ModalData) => {
  const { mutateAsync } = useMutation({
    mutationFn: api.user.editUser,
    meta: {
      invalidate: [['user'], ['user-overview']],
    },
    onSuccess: () => {
      closeModal(modalName);
    },
  });

  const formSchema = useMemo(
    () =>
      z.object({
        username: z
          .string()
          .trim()
          .min(1, m.form_error_required())
          .max(64, m.form_error_max_len({ length: 64 }))
          .regex(patternSafeUsernameCharacters, m.form_error_forbidden_char())
          .refine((value) => {
            if (value === user.username) return true;
            return !reservedUsernames.includes(value);
          }, m.form_error_username_taken()),
        email: z
          .email()
          .trim()
          .min(1, m.form_error_required())
          .refine((value) => {
            // ignore default value
            if (value === user.email) return true;
            if (isPresent(reservedEmails)) {
              return !reservedEmails.includes(value.toLowerCase());
            }
            return true;
          }, m.form_error_email_reserved()),
        last_name: z.string().trim().min(1, m.form_error_required()),
        first_name: z.string().trim().min(1, m.form_error_required()),
        phone: z.string().trim(),
      }),
    [reservedEmails, user.email, reservedUsernames, user.username],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      email: user.email,
      first_name: user.first_name,
      last_name: user.last_name,
      phone: user.phone ?? '',
      username: user.username,
    }),
    [user.email, user.first_name, user.last_name, user.phone, user.username],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const body = removeEmptyStrings({ ...cloneDeep(user), ...value });
      await mutateAsync({
        body,
        username: user.username,
      });
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
          <AppText font={TextStyle.TBodySm500}>{m.modal_edit_user_login_pref()}</AppText>
          <SizedBox height={ThemeSpacing.Lg} />
          <EvenSplit parts={2}>
            <form.AppField name="username">
              {(field) => <field.FormInput required label={m.form_label_username()} />}
            </form.AppField>
            <form.AppField name="email">
              {(field) => <field.FormInput required label={m.form_label_email()} />}
            </form.AppField>
          </EvenSplit>
          <Divider spacing={ThemeSpacing.Xl} />
          <AppText font={TextStyle.TBodySm500}>{m.modal_edit_user_account()}</AppText>
          <SizedBox height={ThemeSpacing.Lg} />
          <EvenSplit>
            <form.AppField name="first_name">
              {(field) => <field.FormInput required label={m.form_label_first_name()} />}
            </form.AppField>
            <form.AppField name="last_name">
              {(field) => <field.FormInput required label={m.form_label_last_name()} />}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="phone">
            {(field) => <field.FormInput label={m.form_label_phone()} />}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          disabled: isSubmitting,
          text: m.controls_cancel(),
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
