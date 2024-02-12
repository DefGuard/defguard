import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import SvgIconCheckmark from '../../../../../../../shared/components/svg/IconCheckmark';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { AuthenticationKeyType } from '../../../../../../../shared/types';
import { useAddAuthorizationKeyModal } from '../../useAddAuthorizationKeyModal';

type FormFields = {
  title: string;
  keyValue: string;
};

type Props = {
  keyType: AuthenticationKeyType;
};

const defaultValues: FormFields = {
  keyValue: '',
  title: '',
};

export const AddAuthenticationKeyForm = ({ keyType }: Props) => {
  const { LL } = useI18nContext();
  const {
    user: { addAuthenticationKey },
  } = useApi();
  const toaster = useToaster();
  const localLL = LL.userPage.authenticationKeys.addModal.keyForm;
  const closeModal = useAddAuthorizationKeyModal((s) => s.close);

  const { mutate, isLoading } = useMutation({
    mutationFn: addAuthenticationKey,
    onSuccess: () => {
      toaster.success(LL.messages.success());
    },
    onError: (e) => {
      toaster.error(LL.messages.error());
      console.error(e);
    },
  });

  const schema = useMemo(
    () =>
      z.object({
        title: z
          .string({
            required_error: LL.form.error.required(),
          })
          .min(1, LL.form.error.required())
          .min(4, LL.form.error.minimumLength()),
        keyValue: z.string({
          required_error: LL.form.error.required(),
        }),
      }),
    [LL.form.error],
  );

  const { handleSubmit, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    mode: 'all',
    defaultValues,
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    mutate({
      key: values.keyValue,
      key_type: keyType,
      name: values.title,
    });
  };

  return (
    <form
      onSubmit={handleSubmit(handleValidSubmit)}
      id="add-authentication-key-modal-form"
    >
      <FormInput
        controller={{ control, name: 'title' }}
        label={localLL.labels.title()}
        placeholder={localLL.placeholders.title()}
        autoComplete="off"
      />
      <FormInput
        controller={{ control, name: 'keyValue' }}
        label={localLL.labels.key()}
        autoComplete="off"
        placeholder={
          keyType === AuthenticationKeyType.SSH
            ? localLL.placeholders.key.ssh()
            : localLL.placeholders.key.gpg()
        }
      />
      <div className="controls">
        <Button
          type="button"
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.common.controls.cancel()}
          onClick={() => closeModal()}
        />
        <Button
          type="submit"
          icon={<SvgIconCheckmark />}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={localLL.submit({ name: keyType.valueOf().toUpperCase() })}
          loading={isLoading}
        />
      </div>
    </form>
  );
};
