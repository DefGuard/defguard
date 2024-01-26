import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import {
  patternNoSpecialChars,
  patternValidWireguardKey,
} from '../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../shared/queries';
import { useEditDeviceModal } from '../../hooks/useEditDeviceModal';

interface Inputs {
  name: string;
  wireguard_pubkey: string;
}

const defaultFormValues: Inputs = {
  name: '',
  wireguard_pubkey: '',
};

export const EditUserDeviceForm = () => {
  const device = useEditDeviceModal((state) => state.device);
  const closeModal = useEditDeviceModal((state) => state.close);
  const { LL } = useI18nContext();

  const zodSchema = useMemo(
    () =>
      z.object({
        name: z
          .string()
          .min(4, LL.form.error.minimumLength())
          .regex(patternNoSpecialChars, LL.form.error.noSpecialChars()),
        wireguard_pubkey: z
          .string()
          .min(44, LL.form.error.invalidKey())
          .max(44, LL.form.error.invalidKey())
          .regex(patternValidWireguardKey, LL.form.error.invalidKey()),
      }),
    [LL.form.error],
  );

  const { control, handleSubmit } = useForm<Inputs>({
    resolver: zodResolver(zodSchema),
    defaultValues: {
      name: device?.name ?? defaultFormValues.name,
      wireguard_pubkey: device?.wireguard_pubkey ?? defaultFormValues.wireguard_pubkey,
    },
    mode: 'all',
  });

  const {
    device: { editDevice },
  } = useApi();

  const toaster = useToaster();
  const queryClient = useQueryClient();

  const { isLoading: editDeviceLoading, mutate } = useMutation(
    [MutationKeys.EDIT_USER_DEVICE],
    editDevice,
    {
      onSuccess: () => {
        toaster.success(LL.modals.editDevice.messages.success());
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        closeModal();
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const onSubmitSuccess: SubmitHandler<Inputs> = (values) => {
    if (device) {
      mutate({ ...device, ...values });
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmitSuccess)}>
      <FormInput
        label={LL.modals.editDevice.form.fields.name.label()}
        controller={{ control, name: 'name' }}
      />
      <FormInput
        label={LL.modals.editDevice.form.fields.publicKey.label()}
        controller={{ control, name: 'wireguard_pubkey' }}
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
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.modals.editDevice.form.controls.submit()}
          loading={editDeviceLoading}
        />
      </div>
    </form>
  );
};
