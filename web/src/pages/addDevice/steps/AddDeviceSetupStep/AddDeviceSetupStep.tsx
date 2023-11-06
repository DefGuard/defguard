import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parser from 'html-react-parser';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormToggle } from '../../../../shared/defguard-ui/components/Form/FormToggle/FormToggle';
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
  const localLL = LL.addDevicePage.steps.setupDevice;
  const toaster = useToaster();
  const {
    device: { addDevice },
  } = useApi();
  const submitRef = useRef<HTMLInputElement | null>(null);
  const userData = useAddDevicePageStore((state) => state.userData);
  const nextStep = useAddDevicePageStore((state) => state.nextStep);
  const nextSubject = useAddDevicePageStore((state) => state.nextSubject);
  const setPageState = useAddDevicePageStore((state) => state.setState);

  const toggleOptions = useMemo(() => {
    const res: ToggleOption<number>[] = [
      {
        text: localLL.options.auto(),
        value: AddDeviceSetupMethod.AUTO,
      },
      {
        text: localLL.options.manual(),
        value: AddDeviceSetupMethod.MANUAL,
      },
    ];
    return res;
  }, [localLL.options]);

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
              localLL.form.errors.name.duplicatedName(),
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
    [LL.form.error, localLL.form.errors.name, userData],
  );

  const { handleSubmit, control } = useForm<FormValues>({
    defaultValues: {
      name: '',
      choice: AddDeviceSetupMethod.AUTO,
      publicKey: '',
    },
    resolver: yupResolver(schema),
    mode: 'all',
  });

  const queryClient = useQueryClient();

  const { mutateAsync: addDeviceMutation, isLoading } = useMutation(
    [MutationKeys.ADD_DEVICE],
    addDevice,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        toaster.success(LL.addDevicePage.messages.deviceAdded());
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
          networks: response.configs.map((c) => ({
            networkName: c.network_name,
            networkId: c.network_id,
          })),
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
          networks: response.configs.map((c) => ({
            networkName: c.network_name,
            networkId: c.network_id,
          })),
        });
      });
    }
  };

  const {
    field: { value: choiceValue },
  } = useController({ control, name: 'choice' });

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      submitRef?.current?.click();
    });

    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject, submitRef]);

  useEffect(() => {
    setPageState({ loading: isLoading });
  }, [isLoading, setPageState]);

  return (
    <Card id="add-device-setup-step" shaded>
      <h2>{localLL.title()}</h2>
      <MessageBox type={MessageBoxType.INFO}>
        {parser(
          localLL.infoMessage({
            addDevicesDocs: externalLink.gitbook.wireguard.addDevices,
          }),
        )}
      </MessageBox>
      <form onSubmit={handleSubmit(validSubmitHandler)}>
        <FormInput
          label={localLL.form.fields.name.label()}
          controller={{ control, name: 'name' }}
        />
        <FormToggle options={toggleOptions} controller={{ control, name: 'choice' }} />
        <FormInput
          label={localLL.form.fields.publicKey.label()}
          controller={{ control, name: 'publicKey' }}
          disabled={choiceValue === AddDeviceSetupMethod.AUTO}
        />
        <input type="submit" className="hidden" ref={submitRef} />
      </form>
    </Card>
  );
};
