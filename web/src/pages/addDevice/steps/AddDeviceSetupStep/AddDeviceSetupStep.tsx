import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parser from 'html-react-parser';
import { useMemo } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../i18n/i18n-react';
import IconDownload from '../../../../shared/components/svg/IconDownload';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ToggleOption } from '../../../../shared/defguard-ui/components/Layout/Toggle/types';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { externalLink } from '../../../../shared/links';
import { MutationKeys } from '../../../../shared/mutations';
import { patternValidWireguardKey } from '../../../../shared/patterns';
import { QueryKeys } from '../../../../shared/queries';
import { generateWGKeys } from '../../../../shared/utils/generateWGKeys';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';
import { AddDeviceSetupMethod } from '../../types';

interface FormValues {
  name: string;
  choice: AddDeviceSetupMethod;
  publicKey?: string;
}

export const AddDeviceSetupStep = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    device: { addDevice },
  } = useApi();

  const userData = useAddDevicePageStore((state) => state.userData);
  const nextStep = useAddDevicePageStore((state) => state.nextStep);

  const toggleOptions = useMemo(() => {
    const res: ToggleOption<number>[] = [
      {
        text: LL.modals.addDevice.web.steps.setup.options.auto(),
        value: AddDeviceSetupMethod.AUTO,
      },
      {
        text: LL.modals.addDevice.web.steps.setup.options.manual(),
        value: AddDeviceSetupMethod.MANUAL,
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
              (value) => !userData?.reservedDevices?.includes(value),
            ),
          publicKey: yup.string().when('choice', {
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            is: (choice: number | undefined) => choice === AddDeviceSetupMethod.MANUAL,
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
    [LL.form.error, LL.modals.addDevice.web.steps.setup.form.errors.name, userData],
  );

  const {
    handleSubmit,
    control,
    formState: { isValid },
  } = useForm<FormValues>({
    defaultValues: {
      name: '',
      choice: AddDeviceSetupMethod.AUTO,
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
    if (!userData) return;
    if (values.choice === AddDeviceSetupMethod.AUTO) {
      const keys = generateWGKeys();
      addDeviceMutation({
        name: values.name,
        wireguard_pubkey: keys.publicKey,
        username: userData.username,
      }).then((response) => {
        nextStep({
          device: response.device,
          publicKey: keys.publicKey,
          privateKey: keys.privateKey,
        });
      });
    } else {
      addDeviceMutation({
        name: values.name,
        wireguard_pubkey: values.publicKey as string,
        username: userData.username,
      }).then((response) => {
        nextStep({
          device: response.device,
          publicKey: values.publicKey as string,
          privateKey: undefined,
        });
      });
    }
  };

  const {
    field: { value: choiceValue },
  } = useController({ control, name: 'choice' });

  return (
    <Card id="add-device-setup-step">
      <MessageBox type={MessageBoxType.INFO}>
        {parser(
          LL.modals.addDevice.web.steps.setup.infoMessage({
            addDevicesDocs: externalLink.gitbook.wireguard.addDevices,
          }),
        )}
      </MessageBox>
      <form onSubmit={handleSubmit(validSubmitHandler)}>
        <FormInput
          label={LL.modals.addDevice.web.steps.setup.form.fields.name.label()}
          controller={{ control, name: 'name' }}
        />
        <FormToggle options={toggleOptions} controller={{ control, name: 'choice' }} />
        <FormInput
          label={LL.modals.addDevice.web.steps.setup.form.fields.publicKey.label()}
          controller={{ control, name: 'publicKey' }}
          disabled={choiceValue === AddDeviceSetupMethod.AUTO}
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
    </Card>
  );
};
