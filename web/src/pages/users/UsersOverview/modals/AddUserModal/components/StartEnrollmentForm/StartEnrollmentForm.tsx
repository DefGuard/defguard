import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ToggleOption } from '../../../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import { useAppStore } from '../../../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { StartEnrollmentRequest } from '../../../../../../../shared/types';
import { useAddUserModal } from '../../hooks/useAddUserModal';

enum EnrollmentMode {
  EMAIL = 1,
  MANUAL = 2,
}

type FormFields = {
  mode: EnrollmentMode;
  email?: string;
};

export const StartEnrollmentForm = () => {
  const { LL } = useI18nContext();
  const {
    user: { startEnrollment, startDesktopActivation },
  } = useApi();

  const smtpEnabled = useAppStore((state) => state.appInfo?.smtp_enabled);

  const user = useAddUserModal((state) => state.user);
  const desktop = useAddUserModal((state) => state.desktop);
  const [nextStep, setModalState, closeModal] = useAddUserModal(
    (state) => [state.nextStep, state.setState, state.close],
    shallow,
  );

  const schema = useMemo(
    () =>
      z
        .object({
          mode: z.nativeEnum(EnrollmentMode),
          email: z.string().optional().or(z.literal('')),
        })
        .superRefine((obj, ctx) => {
          if (obj.mode === EnrollmentMode.EMAIL) {
            if (!obj.email || obj.email.length === 0) {
              ctx.addIssue({
                code: z.ZodIssueCode.custom,
                message: LL.form.error.required(),
                path: ['email'],
              });
            }
            if (!z.string().trim().email().safeParse(obj.email).success) {
              ctx.addIssue({
                code: z.ZodIssueCode.custom,
                message: LL.form.error.invalid(),
                path: ['email'],
              });
            }
          }
        }),
    [LL.form.error],
  );

  const { handleSubmit, control, watch, trigger } = useForm<FormFields>({
    resolver: zodResolver(schema),
    mode: 'all',
    defaultValues: {
      email: user?.email ?? '',
      mode: smtpEnabled ? EnrollmentMode.EMAIL : EnrollmentMode.MANUAL,
    },
  });

  const {
    field: { value: choiceValue },
  } = useController({ control, name: 'mode' });

  const toaster = useToaster();

  const { mutate: startDesktopMutate, isPending: startDesktopLoading } = useMutation({
    mutationFn: startDesktopActivation,
    onSuccess: (res) => {
      toaster.success(LL.modals.startEnrollment.messages.successDesktop());
      if (choiceValue === EnrollmentMode.EMAIL) {
        closeModal();
      } else {
        setModalState({ tokenResponse: res });
        nextStep();
      }
    },
    onError: (err) => {
      console.error(err);
      toaster.error(LL.modals.startEnrollment.messages.errorDesktop());
    },
  });

  const { mutate: startEnrollmentMutate, isPending: startEnrollmentLoading } =
    useMutation({
      mutationFn: startEnrollment,
      onSuccess: (res) => {
        toaster.success(LL.modals.startEnrollment.messages.success());
        if (choiceValue === EnrollmentMode.EMAIL) {
          closeModal();
        } else {
          setModalState({ tokenResponse: res });
          nextStep();
        }
      },
      onError: (err) => {
        console.error(err);
        toaster.error(LL.modals.startEnrollment.messages.error());
      },
    });

  const onSubmit: SubmitHandler<FormFields> = (data) => {
    if (user) {
      const requestData: StartEnrollmentRequest = {
        username: user.username,
        email: data.email,
        send_enrollment_notification: data.mode === EnrollmentMode.EMAIL,
      };
      if (desktop) {
        startDesktopMutate(requestData);
      } else {
        startEnrollmentMutate(requestData);
      }
    }
  };

  const toggleOptions = useMemo(() => {
    const res: ToggleOption<number>[] = [
      {
        text: LL.modals.startEnrollment.form.mode.options.email(),
        value: EnrollmentMode.EMAIL,
        disabled: !smtpEnabled,
      },
      {
        text: LL.modals.startEnrollment.form.mode.options.manual(),
        value: EnrollmentMode.MANUAL,
      },
    ];
    return res;
  }, [LL.modals.startEnrollment.form.mode.options, smtpEnabled]);

  useEffect(() => {
    const sub = watch((_, { name }) => {
      if (name === 'mode') {
        void trigger('email');
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [watch, trigger]);

  return (
    <form
      id="enrollment-start-form"
      data-testid="start-enrollment-form"
      onSubmit={void handleSubmit(onSubmit)}
    >
      {!smtpEnabled && (
        <MessageBox
          style={{ marginBottom: 20 }}
          type={MessageBoxType.WARNING}
          message={LL.modals.startEnrollment.form.smtpDisabled()}
        />
      )}
      <FormToggle options={toggleOptions} controller={{ control, name: 'mode' }} />
      <FormInput
        label={LL.modals.startEnrollment.form.email.label()}
        controller={{ control, name: 'email' }}
        disabled={choiceValue === EnrollmentMode.MANUAL}
      />

      <div className="controls">
        <Button
          type="button"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.form.cancel()}
          className="cancel"
          onClick={() => closeModal()}
        />
        <Button
          type="submit"
          text={
            desktop
              ? LL.modals.startEnrollment.form.submitDesktop()
              : LL.modals.startEnrollment.form.submit()
          }
          styleVariant={ButtonStyleVariant.PRIMARY}
          size={ButtonSize.LARGE}
          loading={startDesktopLoading || startEnrollmentLoading}
        />
      </div>
    </form>
  );
};
