import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import {
  patternAtLeastOneDigit,
  patternAtLeastOneLowerCaseChar,
  patternAtLeastOneSpecialChar,
  patternAtLeastOneUpperCaseChar,
} from '../../../../../shared/patterns';

interface Inputs {
  new_password: string;
  repeat: string;
}
export const ChangePasswordForm = () => {
  const logout = useAuthStore((state) => state.resetState);
  const currentUser = useAuthStore((state) => state.user);
  const setModalState = useModalStore((state) => state.setChangePasswordModal);
  const modalState = useModalStore((state) => state.changePasswordModal);
  const toaster = useToaster();
  const { LL } = useI18nContext();
  const schema = useMemo(
    () =>
      yup
        .object({
          new_password: yup
            .string()
            .min(8, LL.form.error.minimumLength())
            .max(32, LL.form.error.maximumLength())
            .matches(patternAtLeastOneDigit, LL.form.error.oneDigit())
            .matches(patternAtLeastOneSpecialChar, LL.form.error.oneSpecial())
            .matches(patternAtLeastOneUpperCaseChar, LL.form.error.oneUppercase())
            .matches(patternAtLeastOneLowerCaseChar, LL.form.error.oneLowercase())
            .required(LL.form.error.required()),
          repeat: yup
            .string()
            .required(LL.form.error.required())
            .test(
              'password-match',
              LL.form.error.repeat(),
              (value, context) => value === context.parent.new_password
            ),
        })
        .required(),
    [LL.form.error]
  );

  const {
    user: { changePassword },
  } = useApi();

  const changePasswordMutation = useMutation(changePassword, {
    mutationKey: [MutationKeys.CHANGE_PASSWORD],
    onSuccess: () => {
      if (modalState.user && modalState.user.username === currentUser?.username) {
        logout();
        toaster.success(LL.modals.changeUserPassword.messages.success());
        setModalState({ user: undefined, visible: false });
      } else {
        toaster.success(LL.modals.changeUserPassword.messages.success());
        setModalState({ user: undefined, visible: false });
      }
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      setModalState({ user: undefined, visible: false });
      console.error(err);
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
    criteriaMode: 'all',
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
        outerLabel={LL.modals.changeUserPassword.form.fields.newPassword.label()}
        controller={{ control, name: 'new_password' }}
        floatingErrors={{
          title: LL.form.floatingErrors.title(),
        }}
        type="password"
      />
      <FormInput
        outerLabel={LL.modals.changeUserPassword.form.fields.confirmPassword.label()}
        controller={{ control, name: 'repeat' }}
        type="password"
      />

      <section className="controls">
        <Button
          size={ButtonSize.BIG}
          text={LL.form.cancel()}
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
          text={LL.modals.changeUserPassword.form.controls.submit()}
        />
      </section>
    </form>
  );
};
