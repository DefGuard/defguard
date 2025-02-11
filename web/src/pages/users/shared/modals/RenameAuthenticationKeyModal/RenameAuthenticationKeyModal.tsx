import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { isUndefined } from 'lodash-es';
import { useCallback, useMemo } from 'react';
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
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useRenameAuthenticationKeyModal } from './useRenameAuthenticationKeyModal';

export const RenameAuthenticationKeyModal = () => {
  const { LL } = useI18nContext();
  const isOpen = useRenameAuthenticationKeyModal((s) => s.visible);
  const keyName = useRenameAuthenticationKeyModal((s) => s.keyData?.name);
  const [close, reset] = useRenameAuthenticationKeyModal(
    (s) => [s.close, s.reset],
    shallow,
  );
  return (
    <ModalWithTitle
      title={`${LL.common.controls.rename()} ${LL.common.key().toLowerCase()} ${keyName}`}
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
    >
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
  const toaster = useToaster();

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

  const queryClient = useQueryClient();

  const onSuccess = useCallback(() => {
    void queryClient.invalidateQueries({
      queryKey: [QueryKeys.FETCH_AUTHENTICATION_KEYS_INFO],
    });
    toaster.success(LL.messages.success());
    closeModal();
  }, [LL.messages, closeModal, queryClient, toaster]);

  const onError = useCallback(
    (e: AxiosError) => {
      toaster.error(LL.messages.error());
      console.error(e);
    },
    [LL.messages, toaster],
  );

  const { mutate: renameYubiKeyMutation, isPending: isLoadingYubikey } = useMutation({
    mutationFn: renameYubikey,
    onSuccess,
    onError,
  });

  const {
    mutate: renameAuthenticationKeyMutation,
    isPending: isLoadingAuthenticationKey,
  } = useMutation({
    mutationFn: renameAuthenticationKey,
    onSuccess,
    onError,
  });

  const {
    handleSubmit,
    control,
    formState: { isValidating },
    setError,
  } = useForm({
    defaultValues: {
      name: keyData?.name ?? '',
    },
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const submitValid: SubmitHandler<FormFields> = (values) => {
    const name = values.name.trim();
    if (name === keyData?.name) {
      setError(
        'name',
        {
          message: LL.form.error.invalid(),
        },
        {
          shouldFocus: true,
        },
      );
      return;
    }
    if (keyData) {
      if (keyData.key_type === 'yubikey') {
        renameYubiKeyMutation({
          id: keyData.id,
          username: keyData.username,
          name,
        });
      } else {
        renameAuthenticationKeyMutation({
          id: keyData.id,
          username: keyData.username,
          name,
        });
      }
    }
  };

  return (
    <form onSubmit={handleSubmit(submitValid)} id="rename-authentication-key-form">
      <FormInput controller={{ control, name: 'name' }} label={`${LL.common.name()}`} />
      <div className="controls">
        <Button
          className="cancel"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => closeModal()}
          text={LL.common.controls.cancel()}
        />
        <Button
          className="submit"
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          disabled={isValidating || isUndefined(keyData)}
          loading={isLoadingAuthenticationKey || isLoadingYubikey}
          text={LL.common.controls.submit()}
        />
      </div>
    </form>
  );
};
