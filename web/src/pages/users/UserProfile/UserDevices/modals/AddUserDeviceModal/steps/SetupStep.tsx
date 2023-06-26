import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parser from 'html-react-parser';
import { useMemo } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../../../../shared/components/Form/FormToggle/FormToggle';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/components/layout/Button/Button';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../../shared/components/layout/MessageBox/MessageBox';
import { ToggleOption } from '../../../../../../../shared/components/layout/Toggle/Toggle';
import { useModalStore } from '../../../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../../shared/mutations';
import { patternValidWireguardKey } from '../../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../../shared/queries';
import { generateWGKeys } from '../../../../../../../shared/utils/generateWGKeys';
import { IconDownload } from '../../../../../../../shared/components/svg';

export enum AddDeviceSetupChoice {
  AUTO_CONFIG = 1,
  MANUAL_CONFIG = 2,
}

interface FormValues {
  name: string;
  choice: AddDeviceSetupChoice;
  publicKey?: string;
}

export const SetupStep = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const setModalState = useModalStore((state) => state.setUserDeviceModal);
  const reservedNames = useModalStore((state) => state.userDeviceModal.reserverdNames);
  const nextStep = useModalStore((state) => state.userDeviceModal.nextStep);
  const {
    device: { addDevice },
  } = useApi();

  const toggleOptions = useMemo(() => {
    const res: ToggleOption<number>[] = [
      {
        text: LL.modals.addDevice.web.steps.setup.options.auto(),
        value: AddDeviceSetupChoice.AUTO_CONFIG,
      },
      {
        text: LL.modals.addDevice.web.steps.setup.options.manual(),
        value: AddDeviceSetupChoice.MANUAL_CONFIG,
      },
    ];
    return res;
  }, [LL.modals.addDevice.web.steps.setup.options]);

  const schema = useMemo(
    () =>
      yup
        .object()
        .shape({
          choice: yup.number().required(),
          name: yup
            .string()
            .min(4, LL.form.error.minimumLength())
            .required(LL.form.error.required())
            .test(
              'is-duplicated',
              LL.modals.addDevice.web.steps.setup.form.errors.name.duplicatedName(),
              (value) => !reservedNames?.includes(value)
            ),
          publicKey: yup.string().when('choice', {
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            //@ts-ignore
            is: (choice: number | undefined) =>
              choice === AddDeviceSetupChoice.MANUAL_CONFIG,
            then: () =>
              yup
                .string()
                .min(44, LL.form.error.minimumLength())
                .max(44, LL.form.error.maximumLength())
                .required(LL.form.error.required())
                .matches(patternValidWireguardKey, LL.form.error.invalid()),
            otherwise: () => yup.string().optional(),
          }),
        })
        .required(),
    [LL.form.error, LL.modals.addDevice.web.steps.setup.form.errors.name, reservedNames]
  );

  const {
    handleSubmit,
    control,
    formState: { isValid },
  } = useForm<FormValues>({
    defaultValues: {
      name: '',
      choice: AddDeviceSetupChoice.AUTO_CONFIG,
      publicKey: '',
    },
    resolver: yupResolver(schema),
    mode: 'all',
  });

  const queryClient = useQueryClient();

  const { mutateAsync: addDeviceMutation, isLoading: addDeviceLoading } = useMutation(
    [MutationKeys.ADD_DEVICE],
    addDevice,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        toaster.success(LL.modals.addDevice.messages.success());
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );

  const user = useUserProfileStore((state) => state.user);

  const validSubmitHandler: SubmitHandler<FormValues> = async (values) => {
    if (!user) return;
    if (values.choice === AddDeviceSetupChoice.AUTO_CONFIG) {
      const keys = generateWGKeys();
      addDeviceMutation({
        name: values.name,
        wireguard_pubkey: keys.publicKey,
        username: user.username,
      }).then((response) => {
        const configs = response.configs.map((c) => {
          c.config.replace('YOUR_PRIVATE_KEY', keys.privateKey);
          return c;
        });
        const device = response.device;
        setModalState({
          configs,
          deviceName: device.name,
          choice: values.choice,
        });
        nextStep();
      });
    } else {
      addDeviceMutation({
        name: values.name,
        wireguard_pubkey: values.publicKey as string,
        username: user.username,
      }).then((response) => {
        // This needs to be replaced with valid key so the wireguard mobile app can consume QRCode without errors
        const configs = response.configs.map((c) => {
          c.config.replace('YOUR_PRIVATE_KEY', values.publicKey as string);
          return c;
        });
        setModalState({
          configs,
          deviceName: values.name,
          choice: values.choice,
        });
        nextStep();
      });
    }
  };

  const {
    field: { value: choiceValue },
  } = useController({ control, name: 'choice' });

  return (
    <>
      <MessageBox type={MessageBoxType.INFO}>
        {parser(LL.modals.addDevice.web.steps.setup.infoMessage())}
      </MessageBox>
      <form onSubmit={handleSubmit(validSubmitHandler)}>
        <FormInput
          outerLabel={LL.modals.addDevice.web.steps.setup.form.fields.name.label()}
          controller={{ control, name: 'name' }}
        />
        <FormToggle options={toggleOptions} controller={{ control, name: 'choice' }} />
        <FormInput
          outerLabel={LL.modals.addDevice.web.steps.setup.form.fields.publicKey.label()}
          controller={{ control, name: 'publicKey' }}
          disabled={choiceValue === AddDeviceSetupChoice.AUTO_CONFIG}
        />
        <div className="controls">
          <Button
            type="submit"
            text={LL.modals.addDevice.web.steps.setup.form.submit()}
            styleVariant={ButtonStyleVariant.PRIMARY}
            size={ButtonSize.BIG}
            disabled={!isValid}
            loading={addDeviceLoading}
            icon={<IconDownload />}
          />
        </div>
      </form>
    </>
  );
};
