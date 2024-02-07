import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { pick } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { passwordValidator } from '../../../../../../../shared/validators/password';
import { useChangeSelfPasswordModal } from '../hooks/useChangeSelfPasswordModal';

type FormFields = {
  old_password: string;
  new_password: string;
  repeat: string;
};

export const ChangeSelfPasswordForm = () => {
  const { LL } = useI18nContext();
  const { changePasswordSelf } = useApi();
  const resetModal = useChangeSelfPasswordModal((state) => state.reset);

  const zodSchema = useMemo(
    () =>
      z
        .object({
          new_password: passwordValidator(LL),
          old_password: z.string().min(1, LL.form.error.required()),
          repeat: z.string().min(1, LL.form.error.required()),
        })
        .refine((val) => {
          return val.new_password === val.repeat;
        }, LL.form.error.repeat()),
    [LL],
  );

  const { control, handleSubmit } = useForm<FormFields>({
    defaultValues: {
      new_password: '',
      old_password: '',
      repeat: '',
    },
    resolver: zodResolver(zodSchema),
    mode: 'all',
    criteriaMode: 'all',
  });

  const toaster = useToaster();

  const { mutate, isLoading } = useMutation({
    mutationFn: changePasswordSelf,
    onSuccess: () => {
      toaster.success(LL.modals.changePasswordSelf.messages.success());
      resetModal();
    },
    onError: (err) => {
      toaster.error(LL.modals.changePasswordSelf.messages.error());
      console.error(err);
    },
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    mutate(pick(values, ['old_password', 'new_password']));
  };

  return (
    <form
      data-testid="change-self-password-form"
      onSubmit={handleSubmit(handleValidSubmit)}
    >
      <FormInput
        controller={{ control, name: 'old_password' }}
        type="password"
        label={LL.modals.changePasswordSelf.form.labels.oldPassword()}
      />
      <FormInput
        controller={{ control, name: 'new_password' }}
        floatingErrors={{
          title: LL.form.floatingErrors.title(),
        }}
        type="password"
        label={LL.modals.changePasswordSelf.form.labels.newPassword()}
      />
      <FormInput
        label={LL.modals.changePasswordSelf.form.labels.repeat()}
        controller={{ control, name: 'repeat' }}
        type="password"
      />
      <div className="controls">
        <Button
          className="cancel"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.modals.changePasswordSelf.controls.cancel()}
          disabled={isLoading}
          onClick={() => resetModal()}
        />
        <Button
          className="submit"
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.modals.changePasswordSelf.controls.submit()}
          loading={isLoading}
        />
      </div>
    </form>
  );
};
