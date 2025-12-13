import { useEffect, useState } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Modal } from '../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
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
      title={'Send test email'}
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
  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await api.mail
        .sendTestEmail({
          email: value.email,
        })
        .finally(() => {
          closeModal(modalNameValue);
        });
    },
  });

  return (
    <>
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {`Check if your SMTP configuration works by sending a test email.`}
      </AppText>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="email">
            {(field) => <field.FormInput required label={m.form_label_email()} />}
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
                  text: m.controls_send_email(),
                  loading: isSubmitting,
                }}
                cancelProps={{
                  text: m.controls_cancel(),
                  disabled: isSubmitting,
                }}
              />
            )}
          </form.Subscribe>
        </form.AppForm>
      </form>
    </>
  );
};
