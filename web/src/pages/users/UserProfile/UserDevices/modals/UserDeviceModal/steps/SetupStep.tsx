import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parser from 'html-react-parser';
import { useMemo } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { IconDownload } from '../../../../../../../shared/components/svg';
import { useUserProfileStore } from '../../../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { externalLink } from '../../../../../../../shared/links';
import { MutationKeys } from '../../../../../../../shared/mutations';
import { patternValidWireguardKey } from '../../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../../shared/queries';
import { generateWGKeys } from '../../../../../../../shared/utils/generateWGKeys';
import { DeviceModalSetupMode, useDeviceModal } from '../../../hooks/useDeviceModal';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonStyleVariant,
  ButtonSize,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ToggleOption } from '../../../../../../../shared/defguard-ui/components/Layout/Toggle/types';

interface FormValues {
  name: string;
  choice: DeviceModalSetupMode;
  publicKey?: string;
}

export const SetupStep = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    device: { addDevice },
  } = useApi();

  const nextStep = useDeviceModal((state) => state.nextStep);

  const userProfile = useUserProfileStore((state) => state.userProfile);

  const user = userProfile?.user;

  const reservedNames = useMemo(
    () => userProfile?.devices.map((d) => d.name) ?? [],
    [userProfile?.devices],
  );

  const toggleOptions = useMemo(() => {
    const res: ToggleOption<number>[] = [
      {
        text: LL.modals.addDevice.web.steps.setup.options.auto(),
        value: DeviceModalSetupMode.AUTO_CONFIG,
      },
      {
        text: LL.modals.addDevice.web.steps.setup.options.manual(),
        value: DeviceModalSetupMode.MANUAL_CONFIG,
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
              (value) => !reservedNames?.includes(value),
            ),
          publicKey: yup.string().when('choice', {
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            //@ts-ignore
            is: (choice: number | undefined) =>
              choice === DeviceModalSetupMode.MANUAL_CONFIG,
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
    [LL.form.error, LL.modals.addDevice.web.steps.setup.form.errors.name, reservedNames],
  );

  const {
    handleSubmit,
    control,
    formState: { isValid },
  } = useForm<FormValues>({
    defaultValues: {
      name: '',
      choice: DeviceModalSetupMode.AUTO_CONFIG,
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
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        toaster.success(LL.modals.addDevice.messages.success());
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const validSubmitHandler: SubmitHandler<FormValues> = async (values) => {
    if (!user) return;
    if (values.choice === DeviceModalSetupMode.AUTO_CONFIG) {
      const keys = generateWGKeys();
      addDeviceMutation({
        name: values.name,
        wireguard_pubkey: keys.publicKey,
        username: user.username,
      }).then((response) => {
        const configs = response.configs.map((c) => {
          c.config = c.config.replaceAll(/YOUR_PRIVATE_KEY/g, keys.privateKey);
          return c;
        });
        const device = response.device;
        nextStep({
          configs,
          deviceName: device.name,
          setupMode: values.choice,
        });
      });
    } else {
      addDeviceMutation({
        name: values.name,
        wireguard_pubkey: values.publicKey as string,
        username: user.username,
      }).then((response) => {
        // This needs to be replaced with valid key so the wireguard mobile app can consume QRCode without errors
        const configs = response.configs.map((c) => {
          c.config = c.config.replace(/YOUR_PRIVATE_KEY/g, values.publicKey as string);
          return c;
        });
        nextStep({
          configs,
          deviceName: values.name,
          setupMode: values.choice,
        });
      });
    }
  };

  const {
    field: { value: choiceValue },
  } = useController({ control, name: 'choice' });

  return (
    <>
      <MessageBox type={MessageBoxType.INFO}>
        {parser(
          LL.modals.addDevice.web.steps.setup.infoMessage({
            addDevicesDocs: externalLink.gitbook.wireguard.addDevices,
          }),
        )}
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
          disabled={choiceValue === DeviceModalSetupMode.AUTO_CONFIG}
        />
        <div className="controls">
          <Button
            type="submit"
            text={LL.modals.addDevice.web.steps.setup.form.submit()}
            styleVariant={ButtonStyleVariant.PRIMARY}
            size={ButtonSize.LARGE}
            disabled={!isValid}
            loading={addDeviceLoading}
            icon={<IconDownload />}
          />
        </div>
      </form>
    </>
  );
};
