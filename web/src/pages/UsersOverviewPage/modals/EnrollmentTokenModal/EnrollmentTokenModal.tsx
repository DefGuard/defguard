import { useStore } from '@tanstack/react-form';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { CopyField } from '../../../../shared/defguard-ui/components/CopyField/CopyField';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenEnrollmentTokenModal } from '../../../../shared/hooks/modalControls/types';
import { useApp } from '../../../../shared/hooks/useApp';

const modalName = ModalName.EnrollmentToken;

type ModalData = OpenEnrollmentTokenModal;

export const EnrollmentTokenModal = () => {
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
      id="enrollment-token-modal"
      title={m.modal_initiate_self_enrollment_title()}
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

const ModalContent = ({ user }: ModalData) => {
  const [sendEmail, setSendEmail] = useState(false);
  const appInfo = useApp((s) => s.appInfo);

  const { data: enrollmentData } = useQuery({
    queryFn: async () => {
      const response = await api.user.startEnrollment({
        username: user.username,
        send_enrollment_notification: false,
      });
      return {
        enrollment_url: response.data.enrollment_url,
        enrollment_token: response.data.enrollment_token,
      };
    },
    queryKey: ['enrollment-token', user.username],
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  const formSchema = useMemo(
    () =>
      z
        .object({
          email: z.string(),
        })
        .superRefine((values, ctx) => {
          if (sendEmail) {
            const result = z
              .email(m.form_error_email())
              .min(1, m.form_error_required())
              .safeParse(values.email);
            if (!result.success) {
              ctx.addIssue({
                code: 'custom',
                path: ['email'],
                message: result.error.issues[0].message,
              });
            }
          }
        }),
    [sendEmail],
  );

  const { mutateAsync: sendEnrollmentEmail } = useMutation({
    mutationFn: async (email: string) => {
      return api.user.startEnrollment({
        username: user.username,
        send_enrollment_notification: true,
        email,
      });
    },
  });

  const form = useAppForm({
    defaultValues: {
      email: user?.email ?? '',
    },
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await sendEnrollmentEmail(value.email);
      closeModal(modalName);
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  useEffect(() => {
    if (!form.state.isPristine) {
      form.validateAllFields('change');
    }
  }, [form.state.isPristine, form.validateAllFields]);

  if (!isPresent(enrollmentData)) {
    return;
  }

  return (
    <>
      <div className="enrollment-info">
        <AppText font={TextStyle.TBodySm500}>
          {m.modal_add_user_enrollment_details()}
        </AppText>
        <SizedBox height={ThemeSpacing.Xs} />
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
          {m.modal_add_user_enrollment_explain()}
        </AppText>
      </div>
      <SizedBox height={ThemeSpacing.Xl2} />
      <CopyField
        copyTooltip={m.misc_clipboard_copy()}
        label={m.modal_add_user_enrollment_form_label_instance_url()}
        data-testid="activation-url-field"
        text={enrollmentData.enrollment_url}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <CopyField
        label={m.modal_add_user_enrollment_form_label_token()}
        copyTooltip={m.misc_clipboard_copy()}
        data-testid="activation-token-field"
        text={enrollmentData.enrollment_token}
      />
      {appInfo.smtp_enabled && (
        <form
          onSubmit={(e) => {
            e.stopPropagation();
            e.preventDefault();
            form.handleSubmit();
          }}
        >
          <SizedBox height={ThemeSpacing.Xl3} />
          <form.AppForm>
            <Checkbox
              text={m.modal_add_user_enrollment_form_label_send()}
              active={sendEmail}
              onClick={() => {
                setSendEmail((s) => !s);
              }}
            />
            {sendEmail && (
              <>
                <SizedBox height={ThemeSpacing.Xl} />
                <form.AppField name="email">
                  {(field) => (
                    <field.FormInput
                      required={sendEmail}
                      label={m.form_label_email()}
                    />
                  )}
                </form.AppField>
              </>
            )}
          </form.AppForm>
        </form>
      )}
      <ModalControls
        submitProps={{
          text: sendEmail ? m.controls_send_email() : m.controls_close(),
          loading: isSubmitting,
          onClick: () => {
            if (sendEmail) {
              form.handleSubmit();
            } else {
              closeModal(modalName);
            }
          },
        }}
      />
    </>
  );
};
