import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { passwordValidator } from '../../../../../shared/validators/password';

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

  const zodSchema = useMemo(
    () =>
      z
        .object({
          new_password: passwordValidator(LL),
          repeat: z.string().min(1, LL.form.error.required()),
        })
        .superRefine((val, ctx) => {
          const { new_password, repeat } = val;
          if (new_password !== repeat) {
            ctx.addIssue({
              path: ['repeat'],
              code: 'custom',
              message: LL.form.error.repeat(),
            });
          }
        }),
    [LL],
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
    resolver: zodResolver(zodSchema),
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
    <form onSubmit={handleSubmit(onValidSubmit)} data-testid="change-password-admin-form">
      <FormInput
        label={LL.modals.changeUserPassword.form.fields.newPassword.label()}
        controller={{ control, name: 'new_password' }}
        floatingErrors={{
          title: LL.form.floatingErrors.title(),
        }}
        type="password"
      />
      <FormInput
        label={LL.modals.changeUserPassword.form.fields.confirmPassword.label()}
        controller={{ control, name: 'repeat' }}
        type="password"
      />

      <section className="controls">
        <Button
          size={ButtonSize.LARGE}
          text={LL.form.cancel()}
          className="cancel"
          onClick={() => setModalState({ user: undefined, visible: false })}
          tabIndex={4}
          type="button"
        />
        <Button
          size={ButtonSize.LARGE}
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
