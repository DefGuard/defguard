import './styles.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import useApi from '../../../../../shared/hooks/useApi';
import { useRenameAuthenticationKeyModal } from './useRenameAuthenticationKeyModal';

export const RenameAuthenticationKeyModal = () => {
  const isOpen = useRenameAuthenticationKeyModal((s) => s.visible);
  const [close, reset] = useRenameAuthenticationKeyModal(
    (s) => [s.close, s.reset],
    shallow,
  );
  return (
    <ModalWithTitle title="Rename key" isOpen={isOpen} onClose={close} afterClose={reset}>
      <ModalContent />
    </ModalWithTitle>
  );
};

type FormFields = {
  name: string;
};

const ModalContent = () => {
  const {
    user: { renameAuthenticationKey, renameYubikey },
  } = useApi();
  const closeModal = useRenameAuthenticationKeyModal((s) => s.close, shallow);
  const keyData = useRenameAuthenticationKeyModal((s) => s.keyData);
  const { LL } = useI18nContext();

  const schema = useMemo(
    () =>
      z.object({
        name: z
          .string()
          .min(1, LL.form.error.required())
          .min(4, LL.form.error.minimumLength()),
      }),
    [LL.form.error],
  );

  const { mutate: renameYubiKeyMutation, isLoading: isLoadingYubikey } = useMutation({
    mutationFn: renameYubikey,
    onSuccess: () => {
      closeModal();
    },
  });

  const {
    mutate: renameAuthenticationKeyMutation,
    isLoading: isLoadingAuthenticationKey,
  } = useMutation({
    mutationFn: renameAuthenticationKey,
    onSuccess: () => {
      closeModal();
    },
  });

  const {
    handleSubmit,
    control,
    formState: { isValidating },
  } = useForm({
    defaultValues: {
      name: keyData?.name ?? '',
    },
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const submitValid: SubmitHandler<FormFields> = (values) => {
    if (keyData) {
      if (keyData.key_type === 'yubikey') {
        renameAuthenticationKeyMutation({
          id: keyData.id,
          username: keyData.username,
          name: values.name,
        });
      } else {
        renameYubiKeyMutation({
          id: keyData.id,
          username: keyData.username,
          name: values.name,
        });
      }
    }
  };

  return (
    <form onSubmit={handleSubmit(submitValid)} id="rename-authentication-key-form">
      <FormInput controller={{ control, name: 'name' }} />
      <div className="controls">
        <Button
          className="cancel"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => closeModal()}
        />
        <Button
          className="submit"
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          disabled={isValidating || isUndefined(keyData)}
          loading={isLoadingAuthenticationKey || isLoadingYubikey}
        />
      </div>
    </form>
  );
};
