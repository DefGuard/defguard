import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import type { StartEnrollmentResponse, User } from '../../../../shared/api/types';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Fold } from '../../../../shared/defguard-ui/components/Fold/Fold';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
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
import { useApp } from '../../../../shared/hooks/useApp';
import { DeliverTokenStep } from './steps/DeliverTokenStep/DeliverTokenStep';

const modalName = ModalName.AddNewDevice;

type DeliveryMethod = 'email' | 'manual';

export const AddNewDeviceModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [user, setUser] = useState<User | null>(null);
  const [enrollmentData, setEnrollmentData] = useState<StartEnrollmentResponse | null>(
    null,
  );

  const handleAfterClose = () => {
    setUser(null);
    setEnrollmentData(null);
  };

  useEffect(() => {
    const openSub = subscribeOpenModal(modalName, (data) => {
      setUser(data);
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
      id="add-new-device-modal"
      title={m.modal_add_new_device_title()}
      isOpen={isOpen}
      onClose={() => closeModal(modalName)}
      afterClose={handleAfterClose}
    >
      {isPresent(user) && !isPresent(enrollmentData) && (
        <EnrollmentChoice user={user} onEnrollmentReady={setEnrollmentData} />
      )}
      {isPresent(enrollmentData) && <DeliverTokenStep enrollmentData={enrollmentData} />}
    </Modal>
  );
};

const EnrollmentChoice = ({
  user,
  onEnrollmentReady,
}: {
  user: User;
  onEnrollmentReady: (data: StartEnrollmentResponse) => void;
}) => {
  const smtpEnabled = useApp((s) => s.appInfo.smtp_enabled);
  const [selected, setSelected] = useState<DeliveryMethod>(() =>
    smtpEnabled ? 'email' : 'manual',
  );

  const { mutateAsync: startClientActivation } = useMutation({
    mutationFn: api.user.startClientActivation,
    onError: (error) => {
      Snackbar.error(m.failed_to_start_enrollment());
      console.error(error);
    },
  });

  const formSchema = useMemo(
    () =>
      z
        .object({
          email: z.string(),
        })
        .superRefine((values, ctx) => {
          if (selected === 'email') {
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
    [selected],
  );

  const form = useAppForm({
    defaultValues: {
      email: user.email ?? '',
    },
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      if (selected === 'manual') {
        const { data } = await startClientActivation({
          username: user.username,
          send_enrollment_notification: false,
        });
        onEnrollmentReady(data);
      } else {
        await startClientActivation({
          username: user.username,
          send_enrollment_notification: true,
          email: value.email,
        });
        Snackbar.success(m.sucessfull_enrollment_email());
        closeModal(modalName);
      }
    },
  });
  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  return (
    <>
      <div className="enrollment-info">
        <AppText font={TextStyle.TBodySm500}>{m.modal_add_new_device_subtitle()}</AppText>
        <SizedBox height={ThemeSpacing.Xs} />
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
          {m.modal_add_new_device_subtitle_description()}
        </AppText>
      </div>
      <SizedBox height={ThemeSpacing.Xl2} />
      <form
        onSubmit={(e) => {
          e.preventDefault();
          e.stopPropagation();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <SectionSelect
            image="token-email"
            radio
            selected={selected === 'email'}
            disabled={!smtpEnabled}
            badgeProps={
              !smtpEnabled
                ? { variant: 'critical', text: m.state_not_configured() }
                : undefined
            }
            title={m.modal_add_new_device_choice_email_title()}
            content={m.modal_add_new_device_choice_email_content()}
            data-testid="add-new-device-email"
            onClick={() => {
              if (smtpEnabled) setSelected('email');
            }}
          >
            <Fold open={selected === 'email'}>
              <SizedBox height={ThemeSpacing.Lg} />
              <form.AppField name="email">
                {(field) => <field.FormInput label={m.form_label_email()} required />}
              </form.AppField>
            </Fold>
          </SectionSelect>
          <SizedBox height={ThemeSpacing.Md} />
          <SectionSelect
            image="token-chat"
            radio
            selected={selected === 'manual'}
            title={m.modal_add_new_device_choice_manual_title()}
            content={m.modal_add_new_device_choice_manual_content()}
            data-testid="add-new-device-manually"
            onClick={() => setSelected('manual')}
          />
          <SizedBox height={ThemeSpacing.Xl2} />
          <Controls>
            <div className="right">
              <Button
                type="button"
                variant="secondary"
                text={m.controls_cancel()}
                onClick={() => closeModal(modalName)}
              />
              <Button
                type="submit"
                text={m.controls_submit()}
                variant="primary"
                loading={isSubmitting}
              />
            </div>
          </Controls>
        </form.AppForm>
      </form>
    </>
  );
};
