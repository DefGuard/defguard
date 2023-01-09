import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
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
  const device = useModalStore((state) => state.editUserDeviceModal.device);
  const setModalsState = useModalStore((state) => state.setState);
  const { LL, locale } = useI18nContext();

  const schema = useMemo(() => {
    return yup
      .object({
        name: yup
          .string()
          .min(4, LL.form.error.minimumLength())
          .matches(patternNoSpecialChars, LL.form.error.noSpecialChars())
          .required(LL.form.error.required()),
        wireguard_pubkey: yup
          .string()
          .min(44, LL.form.error.invalidKey())
          .max(44, LL.form.error.invalidKey())
          .required(LL.form.error.required())
          .matches(patternValidWireguardKey, LL.form.error.invalidKey()),
      })
      .required();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locale]);

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
        toaster.success(LL.modals.editDevice.messages.success());
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        setModalsState({ editUserDeviceModal: { visible: false } });
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
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
        outerLabel={LL.modals.editDevice.form.fields.name.label()}
        controller={{ control, name: 'name' }}
      />
      <FormInput
        outerLabel={LL.modals.editDevice.form.fields.publicKey.label()}
        controller={{ control, name: 'wireguard_pubkey' }}
      />
      <div className="controls">
        <Button
          type="button"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.form.cancel()}
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
          text={LL.modals.editDevice.form.controls.submit()}
          loading={editDeviceLoading}
        />
      </div>
    </form>
  );
};
