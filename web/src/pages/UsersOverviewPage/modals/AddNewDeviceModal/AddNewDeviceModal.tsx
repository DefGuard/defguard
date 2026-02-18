import './style.scss';
import { useEffect, useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import type { StartEnrollmentResponse, User } from '../../../../shared/api/types';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { FieldError } from '../../../../shared/defguard-ui/components/FieldError/FieldError';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../shared/form';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../../../shared/hooks/useApp';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { DeliveryTokenStep } from './steps/DeliveryTokenStep/DeliveryTokenStep';

const modalName = ModalName.AddNewDevice;

type DeliveryMethod = 'email' | 'manual';

export const AddNewDeviceModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [user, setUser] = useState<User | null>(null);
  const [enrollmentData, setEnrollmentData] = useState<StartEnrollmentResponse | null>(null);

  const handleClose = () => {
    setOpen(false);
  };

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
      onClose={handleClose}
      afterClose={handleAfterClose}
    >
      {isPresent(enrollmentData) ? (
        <DeliveryTokenStep enrollmentData={enrollmentData} onClose={handleClose} />
      ) : (
        isPresent(user) && (
          <EnrollmentChoice
            onClose={handleClose}
            user={user}
            onEnrollmentReady={setEnrollmentData}
          />
        )
      )}
    </Modal>
  );
};

const EnrollmentChoice = ({
  onClose,
  user,
  onEnrollmentReady,
}: {
  onClose: () => void;
  user: User;
  onEnrollmentReady: (data: StartEnrollmentResponse) => void;
}) => {
  const smtpEnabled = useApp((s) => s.appInfo.smtp_enabled);
  const [selected, setSelected] = useState<DeliveryMethod | null>(null);
  const [submitAttempted, setSubmitAttempted] = useState(false);

  const { mutateAsync: startClientActivation, isPending } = useMutation({
    mutationFn: api.user.startClientActivation,
    onError: (error) => {
      Snackbar.error(m.failed_to_start_enrollment());
      console.error(error);
    },
  });

  const form = useAppForm({
    defaultValues: {
      email: user.email ?? '',
    },
    onSubmit: async ({ value }) => {
      if (!isPresent(selected)) return;
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
        onClose();
      }
    },
  });

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
          {selected === 'email' && (
            <form.AppField name="email">
              {(field) => <field.FormInput label={m.form_label_email()} required />}
            </form.AppField>
          )}
        </SectionSelect>
      </form.AppForm>
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
      {submitAttempted && !isPresent(selected) && (
        <>
          <SizedBox height={ThemeSpacing.Sm} />
          <FieldError error={m.modal_add_new_device_error_no_option()} />
        </>
      )}
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          onClick: onClose,
        }}
        submitProps={{
          text: m.controls_submit(),
          loading: isPending,
          onClick: () => {
            setSubmitAttempted(true);
            if (!isPresent(selected)) return;
            form.handleSubmit();
          },
        }}
      />
    </>
  );
};
