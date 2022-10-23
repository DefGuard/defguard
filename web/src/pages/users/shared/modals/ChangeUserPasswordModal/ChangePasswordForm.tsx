import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import * as yup from 'yup';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../../shared/mutations';
import {
  patternAtLeastOneDigit,
  patternAtLeastOneLowerCaseChar,
  patternAtLeastOneSpecialChar,
  patternAtLeastOneUpperCaseChar,
} from '../../../../../shared/patterns';
import { toaster } from '../../../../../shared/utils/toaster';

interface Inputs {
  new_password: string;
  repeat: string;
}
export const ChangePasswordForm = () => {
  const logout = useAuthStore((state) => state.logOut);
  const currentUser = useAuthStore((state) => state.user);
  const { t } = useTranslation('en');
  const setModalState = useModalStore((state) => state.setChangePasswordModal);
  const modalState = useModalStore((state) => state.changePasswordModal);

  const schema = useMemo(
    () =>
      yup
        .object({
          new_password: yup
            .string()
            .min(8, t('form.errors.minimumLength', { length: 8 }))
            .max(32, t('form.errors.maximumLength', { length: 32 }))
            .matches(patternAtLeastOneDigit, t('form.errors.atLeastOneDigit'))
            .matches(
              patternAtLeastOneSpecialChar,
              t('form.errors.atLeastOneSpecialChar')
            )
            .matches(
              patternAtLeastOneUpperCaseChar,
              t('form.errors.atLeastOneUpperCaseChar')
            )
            .matches(
              patternAtLeastOneLowerCaseChar,
              t('form.errors.atLeastOneLowerCaseChar')
            )
            .required(t('form.errors.required')),
          repeat: yup
            .string()
            .required(t('form.errors.required'))
            .test(
              'password-match',
              'Does not match with new password',
              (value, context) => value === context.parent.new_password
            ),
        })
        .required(),
    [t]
  );

  const {
    user: { changePassword },
  } = useApi();
  const changePasswordMutation = useMutation(changePassword, {
    mutationKey: [MutationKeys.CHANGE_PASSWORD],
    onSuccess: () => {
      if (
        modalState.user &&
        modalState.user.username === currentUser?.username
      ) {
        logout();
        setModalState({ user: undefined, visible: false });
      } else {
        setModalState({ user: undefined, visible: false });
      }
      toaster.success('Password changed.');
    },
    onError: (err) => {
      console.error(err);
      toaster.error('Error occured.');
      setModalState({ user: undefined, visible: false });
    },
  });

  const {
    control,
    handleSubmit,
    formState: { isValid },
  } = useForm<Inputs>({
    defaultValues: {
      new_password: '',
      repeat: '',
    },
    resolver: yupResolver(schema),
    mode: 'all',
  });

  const onValidSubmit: SubmitHandler<Inputs> = (values) => {
    if (modalState.user?.username) {
      changePasswordMutation.mutate({
        new_password: values.new_password,
        username: modalState.user.username,
      });
    }
  };

  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput
        outerLabel="New password"
        controller={{ control, name: 'new_password' }}
        type="password"
      />
      <FormInput
        outerLabel="Repeat password"
        controller={{ control, name: 'repeat' }}
        type="password"
      />

      <section className="controls">
        <Button
          size={ButtonSize.BIG}
          text="Cancel"
          className="cancel"
          onClick={() => setModalState({ user: undefined, visible: false })}
          tabIndex={4}
          type="button"
        />
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          type="submit"
          disabled={!isValid}
          loading={changePasswordMutation.isLoading}
          tabIndex={5}
          text="Save new password"
        />
      </section>
    </form>
  );
};
