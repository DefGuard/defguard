import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
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
import { useUserProfileStore } from '../../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';

interface FormInputs {
  name: string;
}

export const AddDeviceDesktopForm = () => {
  const {
    device: { addDevice },
  } = useApi();
  const { LL, locale } = useI18nContext();
  const user = useUserProfileStore((state) => state.userProfile?.user);
  const toaster = useToaster();
  const setModalsState = useModalStore((state) => state.setState);
  const schema = useMemo(
    () =>
      yup
        .object()
        .shape({
          name: yup
            .string()
            .required(LL.form.error.required())
            .min(4, LL.form.error.minimumLength()),
        })
        .required(),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [locale]
  );

  const { mutateAsync, isLoading } = useMutation([MutationKeys.ADD_DEVICE], addDevice, {
    onSuccess: () => {
      toaster.success(LL.modals.addDevice.messages.success());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      setModalsState({ addDeviceDesktopModal: { visible: false } });
      console.error(err);
    },
  });
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
      }).then(async () => {
        toaster.error('FIXME');
        // const configWithSecret = config.replace('YOUR_PRIVATE_KEY', keys.privateKey);
        // const appDir = fs.BaseDirectory.AppData;
        // const dirExists = await fs.exists('wg', { dir: appDir });
        // if (!dirExists) {
        //   await fs.createDir('wg', { dir: appDir });
        // }
        // await fs.writeTextFile('wg/device.conf', configWithSecret, {
        //   dir: appDir,
        // });
      });
      setModalsState({ addDeviceDesktopModal: { visible: false } });
    }
  };
  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput
        controller={{ control, name: 'name' }}
        outerLabel={LL.modals.addDevice.desktop.form.fields.name.label()}
      />
      <div className="controls">
        <Button
          type="button"
          text={LL.form.cancel()}
          onClick={() => setModalsState({ addDeviceDesktopModal: { visible: false } })}
        />
        <Button
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.modals.addDevice.desktop.form.submit()}
          disabled={!isValid}
          loading={isLoading}
        />
      </div>
    </form>
  );
};
