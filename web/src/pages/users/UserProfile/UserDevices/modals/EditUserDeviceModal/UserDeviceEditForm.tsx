import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import * as yup from 'yup';

import { FormInput } from '../../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import {
  patternNoSpecialChars,
  patternValidWireguardKey,
} from '../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../shared/queries';

interface Inputs {
  name: string;
  wireguard_pubkey: string;
}

const defaultFormValues: Inputs = {
  name: '',
  wireguard_pubkey: '',
};

export const EditUserDeviceForm = () => {
  const { t } = useTranslation('en');
  const device = useModalStore((state) => state.editUserDeviceModal.device);
  const setModalsState = useModalStore((state) => state.setState);

  const schema = useMemo(() => {
    return yup
      .object({
        name: yup
          .string()
          .min(4, t('form.errors.minimumLength', { length: 4 }))
          .matches(patternNoSpecialChars, t('form.errors.noSpecialChars'))
          .required(t('form.errors.required')),
        wireguard_pubkey: yup
          .string()
          .min(44, t('form.errors.invalidKey'))
          .max(44, t('form.errors.invalidKey'))
          .required(t('form.errors.required'))
          .matches(patternValidWireguardKey, t('form.errors.invalidKey')),
      })
      .required();
  }, [t]);

  const { control, handleSubmit } = useForm<Inputs>({
    resolver: yupResolver(schema),
    defaultValues: {
      name: device?.name ?? defaultFormValues.name,
      wireguard_pubkey:
        device?.wireguard_pubkey ?? defaultFormValues.wireguard_pubkey,
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
        toaster.success('Device updated.');
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        setModalsState({ editUserDeviceModal: { visible: false } });
      },
      onError: (err) => {
        toaster.error('Error ocurred.');
        console.error(err);
      },
    }
  );

  const onSubmitSuccess: SubmitHandler<Inputs> = (values) => {
    if (device) {
      mutate({ ...device, ...values });
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmitSuccess)}>
      <FormInput
        outerLabel="Device Name"
        controller={{ control, name: 'name' }}
      />
      <FormInput
        outerLabel="Device Public Key (Wireguard)"
        controller={{ control, name: 'wireguard_pubkey' }}
      />
      <div className="controls">
        <Button
          type="button"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          text="Cancel"
          className="cancel"
          onClick={() =>
            setModalsState({
              editUserDeviceModal: { visible: false, device: undefined },
            })
          }
        />
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Edit device"
          loading={editDeviceLoading}
        />
      </div>
    </form>
  );
};
