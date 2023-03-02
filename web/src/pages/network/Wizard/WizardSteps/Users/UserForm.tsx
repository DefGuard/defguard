import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import React, { useEffect } from 'react';
import { Controller, SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import Select from 'react-select';
import * as yup from 'yup';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import { Location, SelectOption } from '../../../../../shared/types';
import { useWizardStore } from '../store';

interface Inputs {
  userName: string;
  email: string;
  locations: SelectOption<Location>[];
}

const UserForm: React.FC = () => {
  const { t } = useTranslation('en');

  const users = useWizardStore((state) => state.users);
  const addUser = useWizardStore((state) => state.addUser);
  const usersCount = useWizardStore((state) => state.users.length);
  const setFormStatus = useWizardStore((state) => state.setFormStatus);
  const locations = useWizardStore((state) => state.locations);
  const networkObserver = useWizardStore((state) => state.network);
  const networkType = networkObserver
    ? networkObserver.getValue().type
    : undefined;
  const onSubmit: SubmitHandler<Inputs> = ({ locations, userName, email }) => {
    addUser({
      locations: locations.map((l): Location => l.value),
      userName,
      email,
    });
  };

  const schema = yup
    .object({
      userName: yup
        .string()
        .required(t('wizard.users.userName.validation.required')),
      email: yup
        .string()
        .required(t('wizard.users.email.validation.required'))
        .test(
          'email',
          t('wizard.users.userName.validation.unique'),
          (value?: string) =>
            !users.map((user) => user.email).includes(value as string)
        ),
      locations: yup.array<SelectOption<Location>>().required(),
    })
    .required();

  const {
    handleSubmit,
    formState: { isSubmitSuccessful },
    control,
    reset,
  } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      email: '',
      userName: '',
      locations: [],
    },
  });

  useEffect(() => {
    if (usersCount > 0) {
      setFormStatus({ 3: true });
    } else {
      setFormStatus({ 3: false });
    }
  }, [setFormStatus, usersCount]);

  useEffect(() => {
    if (isSubmitSuccessful) {
      reset(
        { userName: '', locations: [], email: '' },
        { keepDirty: false, keepTouched: false }
      );
    }
  }, [isSubmitSuccessful, reset]);

  return (
    <div className="user-form">
      <form onSubmit={handleSubmit(onSubmit)}>
        <h2>New user credentials:</h2>
        <FormInput
          controller={{ control, name: 'userName' }}
          required
          placeholder={t('wizard.users.userName.placeholder')}
        />
        <FormInput
          controller={{ control, name: 'email' }}
          placeholder={t('wizard.users.email.placeholder')}
          required
        />
        {networkType === 'regular' ? null : (
          <Controller
            name="locations"
            control={control}
            defaultValue={[]}
            render={({ field }) => (
              <Select
                value={field.value}
                isMulti={true}
                ref={field.ref}
                className="custom-select form"
                classNamePrefix="rs"
                options={locations.map(
                  (l): SelectOption<Location> => ({ value: l, label: l.name })
                )}
                onChange={(val) => field.onChange(val)}
                placeholder="Access Type"
              />
            )}
          />
        )}
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.LINK}
          type="submit"
          text={t('wizard.users.submit')}
        />
      </form>
    </div>
  );
};

export default UserForm;
