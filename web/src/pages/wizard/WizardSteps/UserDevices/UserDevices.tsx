import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  SubmitErrorHandler,
  SubmitHandler,
  useFieldArray,
  useForm,
} from 'react-hook-form';
import { useNavigate } from 'react-router';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../shared/components/Form/FormSelect/FormSelect';
import Button, {
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { Card } from '../../../../shared/components/layout/Card/Card';
import { SelectStyleVariant } from '../../../../shared/components/layout/Select/Select';
import { IconTrash } from '../../../../shared/components/svg';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';
import { ImportedDevice, SelectOption } from '../../../../shared/types';
import { useWizardStore } from '../store';

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
  const navigate = useNavigate();
  const submitRef = useRef<HTMLButtonElement | null>(null);
  const {
    user: { getUsers },
    network: { createUserDevices },
  } = useApi();
  const toaster = useToaster();
  const [devices, setFormStatus, formSubmissionSubject] = useWizardStore(
    (state) => [state.devices, state.setFormStatus, state.formSubmissionSubject],
    shallow
  );
  const { mutateAsync: createUserDevicesMutation } = useMutation(
    [MutationKeys.CREATE_USER_DEVICES],
    createUserDevices,
    {
      onSuccess: async () => {
        toaster.success(LL.wizard.locations.form.messages.devicesCreated());
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
      await createUserDevicesMutation({ devices });
      setFormStatus({ [formId]: true });
    },
    [formId, setFormStatus, createUserDevicesMutation]
  );
  const onInvalidSubmit: SubmitErrorHandler<FormInputs> = () => {
    setFormStatus({ 3: false });
  };

  useEffect(() => {
    const sub = formSubmissionSubject.subscribe((stepId) => {
      if (stepId === formId) {
        submitRef.current?.click();
      }
    });
    return () => sub.unsubscribe();
  }, [formId, formSubmissionSubject]);

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

  const { control, handleSubmit } = useForm<FormInputs>({
    defaultValues: { devices: devices.map(deviceToFormValues) },
    resolver: yupResolver(schema),
  });

  const { fields, remove } = useFieldArray({
    control,
    name: 'devices',
  });

  return (
    <section className="user-devices">
      <Card>
        <form onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}>
          {fields.map((device, index) => (
            <div className="device-form" key={device.id}>
              <div>
                <label>{LL.wizard.locations.form.ip()}</label>
                <p>{device.wireguard_ip}</p>
              </div>
              <div>
                <FormInput
                  controller={{ control, name: `devices.${index}.name` }}
                  outerLabel={LL.wizard.locations.form.name()}
                />
              </div>
              <div>
                <FormSelect
                  styleVariant={SelectStyleVariant.WHITE}
                  options={userOptions}
                  controller={{ control, name: `devices.${index}.user_id` }}
                  outerLabel={LL.wizard.locations.form.user()}
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
