import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parser from 'html-react-parser';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

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
import { trimObjectStrings } from '../../../../shared/utils/trimObjectStrings';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';
import {
  AddDeviceNavigationEvent,
  AddDeviceStep,
  AddNativeWgDeviceMode,
} from '../../types';

export const AddDeviceSetupStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.addDevicePage.steps.setupDevice;
  const toaster = useToaster();
  const {
    device: { addDevice },
  } = useApi();
  const submitRef = useRef<HTMLInputElement | null>(null);
  const userData = useAddDevicePageStore((state) => state.userData);
  const [setStep, navSubject] = useAddDevicePageStore(
    (state) => [state.setStep, state.navigationSubject],
    shallow,
  );

  const toggleOptions = useMemo(() => {
    const res: ToggleOption<AddNativeWgDeviceMode>[] = [
      {
        text: localLL.options.auto(),
        value: AddNativeWgDeviceMode.AUTO,
      },
      {
        text: localLL.options.manual(),
        value: AddNativeWgDeviceMode.MANUAL,
      },
    ];
    return res;
  }, [localLL.options]);

  const zodSchema = useMemo(
    () =>
      z
        .object({
          choice: z.nativeEnum(AddNativeWgDeviceMode),
          name: z
            .string()
            .min(4, LL.form.error.minimumLength())
            .refine((val) => !userData?.reservedDevices?.includes(val), {
              message: localLL.form.errors.name.duplicatedName(),
            }),
          publicKey: z.string(),
        })
        .superRefine((val, ctx) => {
          const { publicKey, choice } = val;
          if (choice === AddNativeWgDeviceMode.MANUAL) {
            const pubKeyRes = z
              .string()
              .min(44, LL.form.error.minimumLength())
              .max(44, LL.form.error.maximumLength())
              .regex(patternValidWireguardKey, LL.form.error.invalid())
              .safeParse(publicKey);
            if (!pubKeyRes.success) {
              ctx.addIssue({
                code: 'custom',
                message: pubKeyRes.error.message,
                path: ['publicKey'],
              });
            }
          } else {
            const pubKeyRes = z.string().safeParse(publicKey);
            if (!pubKeyRes.success) {
              ctx.addIssue({
                code: 'custom',
                path: ['publicKey'],
              });
            }
          }
        }),
    [LL.form.error, localLL.form.errors.name, userData?.reservedDevices],
  );

  type FormFields = z.infer<typeof zodSchema>;

  const { handleSubmit, control } = useForm<FormFields>({
    defaultValues: {
      name: '',
      choice: AddNativeWgDeviceMode.AUTO,
      publicKey: '',
    },
    resolver: zodResolver(zodSchema),
    mode: 'all',
  });

  const queryClient = useQueryClient();

  const { mutateAsync: addDeviceMutation } = useMutation({
    mutationFn: addDevice,
    mutationKey: [MutationKeys.ADD_DEVICE],
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_USER_PROFILE],
      });
      void queryClient.invalidateQueries({
        queryKey: ['user'],
      });
      toaster.success(LL.addDevicePage.messages.deviceAdded());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const validSubmitHandler: SubmitHandler<FormFields> = (values) => {
    if (!userData) return;
    values = trimObjectStrings(values);
    if (values.choice === AddNativeWgDeviceMode.AUTO) {
      const keys = generateWGKeys();
      void addDeviceMutation({
        name: values.name,
        wireguard_pubkey: keys.publicKey,
        username: userData.username,
      }).then((response) => {
        setStep(AddDeviceStep.NATIVE_CONFIGURATION, {
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
      void addDeviceMutation({
        name: values.name,
        wireguard_pubkey: values.publicKey,
        username: userData.username,
      }).then((response) => {
        setStep(AddDeviceStep.NATIVE_CONFIGURATION, {
          device: response.device,
          publicKey: values.publicKey,
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
    const sub = navSubject.subscribe((event) => {
      if (event === AddDeviceNavigationEvent.NEXT) {
        submitRef.current?.click();
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [navSubject]);

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
          disabled={choiceValue === AddNativeWgDeviceMode.AUTO}
        />
        <input type="submit" className="hidden" ref={submitRef} />
      </form>
    </Card>
  );
};
