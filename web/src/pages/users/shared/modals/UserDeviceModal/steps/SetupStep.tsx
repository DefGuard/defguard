import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { FormInput } from '../../../../../../shared/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../../../shared/components/Form/FormToggle/FormToggle';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { ToggleOption } from '../../../../../../shared/components/layout/Toggle/Toggle';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../../../shared/hooks/store/useUserProfileV2Store';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { patternValidWireguardKey } from '../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../shared/queries';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';

export enum AddDeviceSetupChoice {
  AUTO_CONFIG = 1,
  MANUAL_CONFIG = 2,
}

interface FormValues {
  name: string;
  choice: AddDeviceSetupChoice;
  publicKey?: string;
}

const toggleOptions: ToggleOption<number>[] = [
  {
    text: 'Generate key pair',
    value: AddDeviceSetupChoice.AUTO_CONFIG,
  },
  {
    text: 'Use my own public key',
    value: AddDeviceSetupChoice.MANUAL_CONFIG,
  },
];

const schema = yup
  .object()
  .shape({
    choice: yup.number().required(),
    name: yup
      .string()
      .min(4, 'Min. 4 characters.')
      .required('Name is required.'),
    publicKey: yup.string().when('choice', (choice, schema) => {
      if (choice === AddDeviceSetupChoice.MANUAL_CONFIG) {
        return schema
          .min(44, 'Key is invalid.')
          .max(44, 'Key is invalid.')
          .required('Key is required.')
          .matches(patternValidWireguardKey, 'Key is invalid.');
      }
      return schema.optional();
    }),
  })
  .required();

export const SetupStep = () => {
  const toaster = useToaster();
  const setModalState = useModalStore((state) => state.setUserDeviceModal);
  const nextStep = useModalStore((state) => state.userDeviceModal.nextStep);
  const {
    device: { addDevice },
  } = useApi();

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

  const { mutateAsync: addDeviceMutation, isLoading: addDeviceLoading } =
    useMutation([MutationKeys.ADD_DEVICE], addDevice, {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        toaster.success('Device added');
      },
      onError: (err) => {
        toaster.error('Adding device failed');
        console.error(err);
      },
    });

  const user = useUserProfileV2Store((state) => state.user);

  const validSubmitHandler: SubmitHandler<FormValues> = async (values) => {
    if (!user) return;
    if (values.choice === AddDeviceSetupChoice.AUTO_CONFIG) {
      const keys = generateWGKeys();
      addDeviceMutation({
        name: values.name,
        wireguard_pubkey: keys.publicKey,
        username: user.username,
      }).then((config) => {
        const res = config.replace('YOUR_PRIVATE_KEY', keys.privateKey);
        setModalState({
          config: res,
          deviceName: values.name,
          choice: values.choice,
        });
        nextStep();
      });
    } else {
      addDeviceMutation({
        name: values.name,
        wireguard_pubkey: values.publicKey as string,
        username: user.username,
      }).then((config) => {
        // This needs to be replaced with valid key so the wireguard mobile app can consume QRCode
        const res = config.replace(
          'YOUR_PRIVATE_KEY',
          values.publicKey as string
        );
        setModalState({
          config: res,
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
        <p>
          You need to configure WireguardVPN on your device, please visit{' '}
          <a href="">documentation</a> if you don&apos;t know how to do it.
        </p>
      </MessageBox>
      <form onSubmit={handleSubmit(validSubmitHandler)}>
        <FormInput
          outerLabel="Device Name"
          controller={{ control, name: 'name' }}
        />
        <FormToggle
          options={toggleOptions}
          controller={{ control, name: 'choice' }}
        />
        <FormInput
          outerLabel="Provide Your Public Key"
          controller={{ control, name: 'publicKey' }}
          disabled={choiceValue === AddDeviceSetupChoice.AUTO_CONFIG}
        />
        <div className="controls">
          <Button
            className="cancel"
            type="button"
            text="Cancel"
            styleVariant={ButtonStyleVariant.STANDARD}
            size={ButtonSize.BIG}
            onClick={() =>
              setModalState({
                visible: false,
                currentStep: 0,
              })
            }
          />
          <Button
            type="submit"
            text="Generate Config"
            styleVariant={ButtonStyleVariant.PRIMARY}
            size={ButtonSize.BIG}
            disabled={!isValid}
            loading={addDeviceLoading}
          />
        </div>
      </form>
    </>
  );
};
