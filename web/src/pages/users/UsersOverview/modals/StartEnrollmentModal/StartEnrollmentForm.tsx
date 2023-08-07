import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import { useMemo, useState } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../../shared/components/Form/FormToggle/FormToggle';
import {
  ActionButton,
  ActionButtonVariant,
} from '../../../../../shared/components/layout/ActionButton/ActionButton';
import { Button } from '../../../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/types';
import { ExpandableCard } from '../../../../../shared/components/layout/ExpandableCard/ExpandableCard';
import { ToggleOption } from '../../../../../shared/components/layout/Toggle/Toggle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { patternValidEmail } from '../../../../../shared/patterns';

enum EnrollmentMode {
  EMAIL = 1,
  MANUAL = 2,
}

interface Inputs {
  mode: EnrollmentMode;
  email?: string;
}

export const StartEnrollmentForm = () => {
  const { LL } = useI18nContext();
  const [enrollmentUrl, setEnrollmentUrl] = useState<string | undefined>(undefined);
  const [enrollmentToken, setEnrollmentToken] = useState<string | undefined>(undefined);
  const {
    user: { startEnrollment },
  } = useApi();

  const formSchema = yup
    .object({
      mode: yup.number().required(),
      email: yup.string().when('mode', {
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        //@ts-ignore
        is: (choice: number | undefined) => choice === EnrollmentMode.EMAIL,
        then: () =>
          yup
            .string()
            .required(LL.form.error.required())
            .matches(patternValidEmail, LL.form.error.invalid()),
        otherwise: () => yup.string().optional(),
      }),
    })
    .required();

  const {
    handleSubmit,
    control,
    formState: { isValid },
  } = useForm<Inputs>({
    resolver: yupResolver(formSchema),
    mode: 'all',
    criteriaMode: 'all',
    defaultValues: {
      email: '',
      mode: EnrollmentMode.EMAIL,
    },
  });

  const {
    field: { value: choiceValue },
  } = useController({ control, name: 'mode' });

  const setModalState = useModalStore((state) => state.setStartEnrollmentModal);
  const user = useModalStore((state) => state.startEnrollmentModal.user);

  const toaster = useToaster();

  const startEnrollmentMutation = useMutation(startEnrollment, {
    onSuccess: () => {
      toaster.success(LL.modals.startEnrollment.messages.success());
      if (choiceValue === EnrollmentMode.EMAIL) {
        setModalState({ visible: false });
      }
    },
    onError: (err) => {
      console.error(err);
      setModalState({ visible: false });
      toaster.error(LL.modals.startEnrollment.messages.error());
    },
  });

  const onSubmit: SubmitHandler<Inputs> = async (data) => {
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
          clipboard
            .write(`Enrollment URL: ${enrollmentUrl}\nToken: ${enrollmentToken}`)
            .then(() => {
              toaster.success(LL.messages.successClipboard());
            })
            .catch((err) => {
              toaster.error(LL.messages.clipboardError());
              console.error(err);
            });
        }}
      />,
    ],
    [enrollmentUrl, enrollmentToken, toaster, LL.messages]
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
            outerLabel={LL.modals.startEnrollment.form.email.label()}
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
              onClick={() => setModalState({ visible: false })}
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
