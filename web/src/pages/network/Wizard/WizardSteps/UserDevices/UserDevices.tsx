import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { SubmitErrorHandler, SubmitHandler, useFieldArray, useForm } from 'react-hook-form';
import useBreakpoint from 'use-breakpoint';
import * as yup from 'yup';
import shallow from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import { Card } from '../../../../../shared/components/layout/Card/Card';
import { Helper } from '../../../../../shared/components/layout/Helper/Helper';
import MessageBox from '../../../../../shared/components/layout/MessageBox/MessageBox';
import { deviceBreakpoints } from '../../../../../shared/constants';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { ImportedDevice, ImportNetworkRequest, SelectOption } from '../../../../../shared/types';
import { useWizardStore } from '../store';
import Button, { ButtonSize, ButtonStyleVariant } from '../../../../../shared/components/layout/Button/Button';
import { IconArrowGrayUp, IconTrash } from '../../../../../shared/components/svg';
import { FormSelect } from '../../../../../shared/components/Form/FormSelect/FormSelect';
import { SelectStyleVariant } from '../../../../../shared/components/layout/Select/Select';
import { QueryKeys } from '../../../../../shared/queries';
import { useNavigate } from 'react-router';

// TODO: cleanup
// type inputNetworkType = 'mesh' | 'regular';

interface Props {
  formId: number;
}

interface DeviceInput extends Omit<ImportedDevice, 'user_id'> {
  user_id: SelectOption<number>;
}
interface FormInputs {
  devices: DeviceInput[];
}

export const UserDevices: React.FC<Props> = ({ formId }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const navigate = useNavigate();
  const submitRef = useRef<HTMLButtonElement | null>(null);
  const {
    user: { getUsers },
    network: { createUserDevices },
  } = useApi();
  const toaster = useToaster();
  const [
    devices,
    setNetwork,
    setState,
    setFormStatus,
    proceedWizardSubject,
    formSubmissionSubject,
  ] = useWizardStore(
    (state) => [
      state.devices,
      state.setNetwork,
      state.setState,
      state.setFormStatus,
      state.proceedWizardSubject,
      state.formSubmissionSubject,
    ],
    shallow
  );
  const { mutateAsync: createUserDevicesMutation } = useMutation(
    [MutationKeys.CREATE_USER_DEVICES],
    createUserDevices,
    {
      onSuccess: async (response) => {
        // TODO: cleanup
        console.log(response);
        toaster.success(LL.wizard.TODO());
        navigate('/admin/network');
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );
  const { LL } = useI18nContext();

  const onValidSubmit: SubmitHandler<FormInputs> = useCallback(
    async (data) => {
      // Set device.user_id
      const devices: ImportedDevice[] = data.devices?.map((d) => ({
        ...d,
        user_id: d.user_id.value,
      }));
      await createUserDevicesMutation({devices});
      setFormStatus({ [formId]: true });
      // proceedWizardSubject.next();
    },
    [formId, setFormStatus, createUserDevicesMutation]
  );
  const onInvalidSubmit: SubmitErrorHandler<FormInputs> = () => {
    setFormStatus({ 3: false });
  };

  // TODO: cleanup
  // const network = networkObserver ? networkObserver.getValue() : undefined;

  // const schema = yup
  //   .object({
  //     type: yup.mixed<inputNetworkType>().oneOf(['mesh', 'regular']).required(),
  //   })
  //   .required();

  // const { handleSubmit, control } = useForm<Inputs>({
  //   resolver: yupResolver(schema),
  //   mode: 'all',
  //   defaultValues: {
  //     name: network?.name ?? '',
  //     type: network?.type ?? 'regular',
  //   },
  // });

  // TODO: use loading?
  // const [save, loading] = useNetworkPageStore(
  //   (state) => [state.saveSubject, state.loading],
  //   shallow
  // )
  useEffect(() => {
    const sub = formSubmissionSubject.subscribe((stepId) => {
      if (stepId === formId) {
        // TODO: cleanup
        // save.next();
        submitRef.current?.click();
      }
    });
    return () => sub.unsubscribe();
    // }, [formId, formSubmissionSubject, save]);
  }, [formId, formSubmissionSubject]);

  // const defaultValues: FormInputs = {
  //   name: '',
  //   endpoint: '',
  //   fileName: '',
  //   config: '',
  // };

  const schema = yup
    .object({
      devices: yup.array().of(
        yup.object({
          name: yup.string().required(LL.form.error.required()),
          user_id: yup.object({
            value: yup.number().min(1),
          }),
        })
      ),
    })
    .required();

  const { data: users, isLoading: usersLoading } = useQuery(
    [QueryKeys.FETCH_USERS],
    getUsers
  );

  const userOptions = useMemo(() => {
    if (!usersLoading && users) {
      return users.map((u) => ({
        key: u.id || -1,
        value: u.id || -1,
        label: u.username || '',
      }));
    }
    return [];
  }, [users, usersLoading]);

  const deviceToFormValues = (device: ImportedDevice): DeviceInput => ({
    ...device,
    user_id: {
      label: users?.find((u) => u.id === device.user_id)?.username || '',
      value: device.user_id,
    },
  });

  const {
    control,
    handleSubmit,
    reset: resetForm,
  } = useForm<FormInputs>({
    defaultValues: { devices: devices.map(deviceToFormValues) },
    resolver: yupResolver(schema),
  });

  const { fields, remove } = useFieldArray({
    control,
    name: 'devices',
  });

  // const { control, handleSubmit, reset, getValues } = useForm<FormInputs>({
  //   defaultValues,
  //   resolver: yupResolver(schema),
  // });

  return (
    <section className="user-devices">
      <header>
        <h2>{LL.networkConfiguration.header()}</h2>
        <Helper>
          <p>PLACEHOLDER</p>
        </Helper>
      </header>
      <Card>
        <form onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}>
          {fields.map((device, index) => (
            <div className="device-form" key={device.id}>
              <div>
                <label>{LL.wizard.TODO()}</label>
                <p>{device.wireguard_ip}</p>
              </div>
              <div>
                <FormInput
                  controller={{ control, name: `devices.${index}.name` }}
                  outerLabel={LL.wizard.TODO()}
                />
              </div>
              <div>
                <FormSelect
                  styleVariant={SelectStyleVariant.WHITE}
                  options={userOptions}
                  controller={{ control, name: `devices.${index}.user_id` }}
                  outerLabel={LL.wizard.TODO()}
                  loading={false}
                  searchable={false}
                  multi={false}
                  disabled={false}
                />
              </div>
              <Button
                icon={<IconTrash />}
                styleVariant={ButtonStyleVariant.ICON}
                onClick={() => remove(index)}
              />
            </div>
          ))}
          <button type="submit" className="hidden" ref={submitRef}></button>
        </form>
      </Card>
    </section>
  );
};
