import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import * as yup from 'yup';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import {
  patternNoSpecialChars,
  patternValidWireguardKey,
} from '../../../../../shared/patterns';
import { QueryKeys } from '../../../../../shared/queries';

interface Inputs {
  name: string;
  wireguard_pubkey: string;
}

const defaultFormValues: Inputs = {
  name: '',
  wireguard_pubkey: '',
};
export const UserDeviceModalForm = () => {
  const { t } = useTranslation('en');
  const modalState = useModalStore((state) => state.userDeviceModal);
  const setModalState = useModalStore((state) => state.setUserDeviceModal);
  const queryClient = useQueryClient();

  const editMode = useMemo(() => {
    if (modalState.device) {
      if (
        modalState.device.name &&
        modalState.device.wireguard_pubkey &&
        modalState.username
      ) {
        return true;
      }
    }
    return false;
  }, [modalState.device, modalState.username]);

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
    defaultValues: editMode
      ? {
          name: modalState.device?.name || '',
          wireguard_pubkey: modalState.device?.wireguard_pubkey || '',
        }
      : defaultFormValues,
    mode: 'all',
  });

  const onSubmitSuccess: SubmitHandler<Inputs> = (values) => {
    if (modalState.username) {
      if (editMode && modalState.device) {
        editDeviceMutation({ ...modalState.device, ...values });
      } else {
        addDeviceMutaion({
          ...values,
          username: modalState.username,
        });
      }
    }
  };

  const {
    device: { addDevice, editDevice },
  } = useApi();

  const toaster = useToaster();

  const onMutationSuccess = () => {
    queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
    setModalState({
      visible: false,
      device: undefined,
      username: undefined,
    });
  };

  const { isLoading: editDeviceLoading, mutate: editDeviceMutation } =
    useMutation([MutationKeys.EDIT_USER_DEVICE], editDevice, {
      onSuccess: () => {
        toaster.success('Device updated.');
        onMutationSuccess();
      },
    });

  const { isLoading: addDeviceLoading, mutate: addDeviceMutaion } = useMutation(
    [MutationKeys.ADD_DEVICE],
    addDevice,
    {
      onSuccess: () => {
        toaster.success('Device added.');
        onMutationSuccess();
      },
    }
  );

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
            setModalState({
              visible: false,
              username: undefined,
              device: undefined,
            })
          }
        />
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Add device"
          loading={addDeviceLoading || editDeviceLoading}
        />
      </div>
    </form>
  );
};
