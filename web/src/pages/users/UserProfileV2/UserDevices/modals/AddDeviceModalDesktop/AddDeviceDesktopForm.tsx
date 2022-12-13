import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { fs } from '@tauri-apps/api';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { FormInput } from '../../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../../../shared/hooks/store/useUserProfileV2Store';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';

const schema = yup
  .object()
  .shape({
    name: yup
      .string()
      .required('Name is required.')
      .min(4, 'At least 4 characters long.'),
  })
  .required();

interface FormInputs {
  name: string;
}

export const AddDeviceDesktopForm = () => {
  const {
    device: { addDevice },
  } = useApi();
  const user = useUserProfileV2Store((state) => state.user);
  const toaster = useToaster();
  const setModalsState = useModalStore((state) => state.setState);
  const { mutateAsync, isLoading } = useMutation(
    [MutationKeys.ADD_DEVICE],
    addDevice,
    {
      onSuccess: () => {
        toaster.success('Device added.');
      },
      onError: (err) => {
        toaster.error('Error ocurred.');
        setModalsState({ addDeviceDesktopModal: { visible: false } });
        console.error(err);
      },
    }
  );
  const {
    control,
    handleSubmit,
    formState: { isValid },
  } = useForm<FormInputs>({
    mode: 'all',
    defaultValues: {
      name: '',
    },
    resolver: yupResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<FormInputs> = (values) => {
    if (user) {
      const keys = generateWGKeys();
      mutateAsync({
        username: user.username,
        name: values.name,
        wireguard_pubkey: keys.publicKey,
      }).then(async (config) => {
        const configWithSecret = config.replace(
          'YOUR_PRIVATE_KEY',
          keys.privateKey
        );
        const appDir = fs.BaseDirectory.AppData;
        const dirExists = await fs.exists('wg', { dir: appDir });
        if (!dirExists) {
          await fs.createDir('wg', { dir: appDir });
        }
        await fs.writeTextFile('wg/device.conf', configWithSecret, {
          dir: appDir,
        });
      });
      setModalsState({ addDeviceDesktopModal: { visible: false } });
    }
  };
  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput controller={{ control, name: 'name' }} outerLabel="Name" />
      <div className="controls">
        <Button
          type="button"
          text="Cancel"
          onClick={() =>
            setModalsState({ addDeviceDesktopModal: { visible: false } })
          }
        />
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Add this device"
          disabled={!isValid}
          loading={isLoading}
        />
      </div>
    </form>
  );
};
