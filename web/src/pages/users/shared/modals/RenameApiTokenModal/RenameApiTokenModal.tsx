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
import { useRenameApiTokenModal } from './useRenameApiTokenModal';

export const RenameApiTokenModal = () => {
  const { LL } = useI18nContext();
  const isOpen = useRenameApiTokenModal((s) => s.visible);
  const keyName = useRenameApiTokenModal((s) => s.tokenData?.name);
  const [close, reset] = useRenameApiTokenModal((s) => [s.close, s.reset], shallow);
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
    user: { renameApiToken },
  } = useApi();
  const closeModal = useRenameApiTokenModal((s) => s.close, shallow);
  const tokenData = useRenameApiTokenModal((s) => s.tokenData);
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
      queryKey: [QueryKeys.FETCH_API_TOKENS_INFO],
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

  const { mutate: renameApiTokenMutation, isPending } = useMutation({
    mutationFn: renameApiToken,
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
      name: tokenData?.name ?? '',
    },
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const submitValid: SubmitHandler<FormFields> = (values) => {
    const name = values.name.trim();
    if (name === tokenData?.name) {
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
    if (tokenData) {
      renameApiTokenMutation({
        id: tokenData.id,
        username: tokenData.username,
        name,
      });
    }
  };
  return (
    <form onSubmit={handleSubmit(submitValid)} id="rename-api-token-form">
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
          disabled={isValidating || isUndefined(tokenData)}
          loading={isPending}
          text={LL.common.controls.submit()}
        />
      </div>
    </form>
  );
};
