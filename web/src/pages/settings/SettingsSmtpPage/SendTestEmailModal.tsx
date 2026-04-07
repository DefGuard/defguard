import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Modal } from '../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';

const modalNameValue = ModalName.SendTestMail;

export const SendTestEmailModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, () => {
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      title={m.settings_smtp_button_send_test_email()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      size="small"
    >
      <ModalContent />
    </Modal>
  );
};

const formSchema = z.object({
  email: z.email(m.form_error_email()).min(1, m.form_error_required()),
});

type FormFields = z.infer<typeof formSchema>;

const defaultValues: FormFields = {
  email: '',
};

const ModalContent = () => {
  const { mutateAsync: sendTestEmail, isPending } = useMutation({
    mutationFn: api.mail.sendTestEmail,
    onSuccess: () => {
      Snackbar.default(m.settings_smtp_test_success());
      closeModal(modalNameValue);
    },
    onError: () => {
      Snackbar.error(m.settings_smtp_test_failed());
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
      await sendTestEmail({
        to: value.email,
      });
    },
  });

  return (
    <>
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {m.settings_smtp_test_email_description()}
      </AppText>
      <SizedBox height={ThemeSpacing.Xl2} />
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="email">
            {(field) => (
              <field.FormInput
                required
                label={m.form_label_email()}
                helper={m.settings_smtp_test_email_helper_email()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.Subscribe
            selector={(s) => ({
              isDefault: s.isDefaultValue || s.isPristine,
              isSubmitting: s.isSubmitting,
            })}
          >
            {({ isSubmitting }) => (
              <ModalControls
                submitProps={{
                  testId: 'submit',
                  text: m.controls_send_email(),
                  loading: isSubmitting || isPending,
                  onClick: () => {
                    form.handleSubmit();
                  },
                }}
                cancelProps={{
                  testId: 'cancel',
                  text: m.controls_cancel(),
                  disabled: isSubmitting || isPending,
                  onClick: () => {
                    closeModal(modalNameValue);
                  },
                }}
              />
            )}
          </form.Subscribe>
        </form.AppForm>
      </form>
    </>
  );
};
