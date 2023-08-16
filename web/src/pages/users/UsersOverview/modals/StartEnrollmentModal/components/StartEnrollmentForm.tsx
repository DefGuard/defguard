import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import { ActionButton } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { ToggleOption } from '../../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import useApi from '../../../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../../../shared/hooks/useClipboard';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { useEnrollmentModalStore } from '../hooks/useEnrollmentModalStore';

enum EnrollmentMode {
  EMAIL = 1,
  MANUAL = 2,
}

type FormFields = {
  mode: EnrollmentMode;
  email?: string;
};

export const StartEnrollmentForm = () => {
  const { writeToClipboard } = useClipboard();
  const closeModal = useEnrollmentModalStore((state) => state.close);
  const user = useEnrollmentModalStore((state) => state.user);
  const { LL } = useI18nContext();
  const [enrollmentUrl, setEnrollmentUrl] = useState<string | undefined>(undefined);
  const [enrollmentToken, setEnrollmentToken] = useState<string | undefined>(undefined);
  const {
    user: { startEnrollment },
  } = useApi();

  const schema = useMemo(
    () =>
      z
        .object({
          mode: z.nativeEnum(EnrollmentMode),
          email: z.string().trim().email(LL.form.error.invalid()).optional(),
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
          }
        }),
    [LL.form.error],
  );

  const {
    handleSubmit,
    control,
    formState: { isValid },
  } = useForm<FormFields>({
    resolver: zodResolver(schema),
    mode: 'all',
    defaultValues: {
      email: '',
      mode: EnrollmentMode.EMAIL,
    },
  });

  const {
    field: { value: choiceValue },
  } = useController({ control, name: 'mode' });

  const toaster = useToaster();

  const startEnrollmentMutation = useMutation(startEnrollment, {
    onSuccess: () => {
      toaster.success(LL.modals.startEnrollment.messages.success());
      if (choiceValue === EnrollmentMode.EMAIL) {
        closeModal();
      }
    },
    onError: (err) => {
      console.error(err);
      toaster.error(LL.modals.startEnrollment.messages.error());
    },
  });

  const onSubmit: SubmitHandler<FormFields> = async (data) => {
    if (user) {
      startEnrollmentMutation
        .mutateAsync({
          username: user.username,
          email: data.email,
          send_enrollment_notification: data.mode === EnrollmentMode.EMAIL,
        })
        .then((response) => {
          setEnrollmentUrl(response.enrollment_url);
          setEnrollmentToken(response.enrollment_token);
        });
    }
  };

  const toggleOptions = useMemo(() => {
    const res: ToggleOption<number>[] = [
      {
        text: LL.modals.startEnrollment.form.mode.options.email(),
        value: EnrollmentMode.EMAIL,
      },
      {
        text: LL.modals.startEnrollment.form.mode.options.manual(),
        value: EnrollmentMode.MANUAL,
      },
    ];
    return res;
  }, [LL.modals.startEnrollment.form.mode.options]);

  const showToken = Boolean(enrollmentUrl && enrollmentToken);

  const getActions = useMemo(
    () => [
      <ActionButton
        type="button"
        key={1}
        variant={ActionButtonVariant.COPY}
        onClick={() => {
          const res = `Enrollment URL: ${enrollmentUrl}\nToken: ${enrollmentToken}`;
          writeToClipboard(res);
        }}
      />,
    ],
    [enrollmentUrl, enrollmentToken, writeToClipboard],
  );

  return (
    <>
      {showToken ? (
        <ExpandableCard
          title={LL.modals.startEnrollment.tokenCard.title()}
          disableExpand={true}
          expanded={true}
          actions={getActions}
        >
          <p>Enrollment URL: {enrollmentUrl}</p>
          <p>Token: {enrollmentToken}</p>
        </ExpandableCard>
      ) : (
        <form data-testid="start-enrollment-form" onSubmit={handleSubmit(onSubmit)}>
          <FormToggle
            options={toggleOptions}
            controller={{ control, name: 'mode' }}
            disabled={showToken}
          />
          <FormInput
            label={LL.modals.startEnrollment.form.email.label()}
            controller={{ control, name: 'email' }}
            disabled={choiceValue === EnrollmentMode.MANUAL || showToken}
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
              text={LL.modals.startEnrollment.form.submit()}
              styleVariant={ButtonStyleVariant.PRIMARY}
              size={ButtonSize.LARGE}
              disabled={!isValid}
              loading={startEnrollmentMutation.isLoading}
            />
          </div>
        </form>
      )}
    </>
  );
};
