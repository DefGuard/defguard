import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosResponse } from 'axios';
import React, { useEffect, useMemo, useState } from 'react';
import { useController, useForm } from 'react-hook-form';
import { SubmitHandler } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { BehaviorSubject, Subject } from 'rxjs';
import {
  debounceTime,
  distinctUntilChanged,
  filter,
  map,
  switchMap,
} from 'rxjs/operators';
import * as yup from 'yup';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import {
  patternAtLeastOneDigit,
  patternAtLeastOneLowerCaseChar,
  patternAtLeastOneSpecialChar,
  patternAtLeastOneUpperCaseChar,
  patternDigitOrLowercase,
  patternNoSpecialChars,
  patternValidEmail,
  patternValidPhoneNumber,
} from '../../../../../shared/patterns';
import { QueryKeys } from '../../../../../shared/queries';

interface Inputs {
  username: string;
  password: string;
  email: string;
  last_name: string;
  first_name: string;
  phone: string;
}

const AddUserForm = () => {
  const { t } = useTranslation('en');
  const {
    user: { addUser, usernameAvailable },
  } = useApi();
  const [usernameSubject] = useState<Subject<string>>(new Subject());
  const [usernamesTaken] = useState<BehaviorSubject<string[]>>(
    new BehaviorSubject<string[]>([])
  );
  const formSchema = useMemo(
    () =>
      yup
        .object({
          username: yup
            .string()
            .required(t('form.errors.required'))
            .matches(patternNoSpecialChars, t('form.errors.noSpecialChars'))
            .matches(patternDigitOrLowercase, t('form.errors.digitOrLowercase'))
            .min(4, t('form.errors.minimumLength', { length: 4 }))
            .test(
              'username-available',
              t('users.form.errors.usernameTaken'),
              (value?: string) =>
                value ? !usernamesTaken.getValue().includes(value) : false
            ),
          password: yup
            .string()
            .min(8, t('form.errors.minimumLength', { length: 8 }))
            .max(32, t('form.errors.maximumLength', { length: 32 }))
            .matches(patternAtLeastOneDigit, t('form.errors.atLeastOneDigit'))
            .matches(
              patternAtLeastOneSpecialChar,
              t('form.errors.atLeastOneSpecialChar')
            )
            .matches(
              patternAtLeastOneUpperCaseChar,
              t('form.errors.atLeastOneUpperCaseChar')
            )
            .matches(
              patternAtLeastOneLowerCaseChar,
              t('form.errors.atLeastOneLowerCaseChar')
            )
            .required(),
          email: yup
            .string()
            .required(t('form.errors.required'))
            .matches(patternValidEmail, t('form.errors.email')),
          last_name: yup
            .string()
            .required(t('form.errors.required'))
            .min(4, t('form.errors.minimumLength', { length: 4 })),
          first_name: yup
            .string()
            .required(t('form.errors.required'))
            .min(4, t('form.errors.minimumLength', { length: 4 })),
          phone: yup
            .string()
            .required(t('form.errors.required'))
            .matches(patternValidPhoneNumber, t('form.errors.phoneNumber')),
        })
        .required(),
    [t, usernamesTaken]
  );
  const {
    handleSubmit,
    control,
    setError,
    formState: { isValid },
  } = useForm<Inputs>({
    resolver: yupResolver(formSchema),
    mode: 'all',
    defaultValues: {
      email: '',
      first_name: '',
      last_name: '',
      password: '',
      phone: '',
      username: '',
    },
  });
  const {
    field: { value: usernameValue },
  } = useController({ control, name: 'username' });
  const queryClient = useQueryClient();
  const setModalState = useModalStore((state) => state.setAddUserModal);
  const toaster = useToaster();
  const addUserMutation = useMutation(addUser, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_USERS]);
      toaster.success('User added.');
      setModalState({ visible: false });
    },
    onError: (err) => {
      console.error(err);
      setModalState({ visible: false });
      toaster.error('Error occured.');
    },
  });

  const onSubmit: SubmitHandler<Inputs> = (data) =>
    addUserMutation.mutate(data);

  useEffect(() => {
    if (usernameSubject) {
      usernameSubject.next(usernameValue);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [usernameSubject, usernameValue]);

  useEffect(() => {
    const subscription = usernameSubject
      .pipe(
        map((s) => s.trim()),
        debounceTime(500),
        distinctUntilChanged(),
        filter((s) => s.length >= 4),
        filter((s) => !usernamesTaken.getValue().includes(s)),
        switchMap((username) =>
          usernameAvailable(username)
            .then((res: AxiosResponse) => {
              if (res.status === 400) {
                usernamesTaken.next([...usernamesTaken.getValue(), username]);
                return false;
              }
              return true;
            })
            .catch(() => {
              usernamesTaken.next([...usernamesTaken.getValue(), username]);
              return false;
            })
        )
      )
      .subscribe((available: boolean) => {
        if (!available) {
          setError(
            'username',
            { message: t('users.form.errors.usernameTaken') },
            { shouldFocus: true }
          );
        }
      });
    return () => {
      subscription.unsubscribe();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [usernameSubject]);

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <div className="row">
        <div className="item">
          <FormInput
            placeholder="User name"
            controller={{ control, name: 'username' }}
            outerLabel="User name"
            required
          />
          <FormInput
            outerLabel="Password"
            placeholder="Password"
            controller={{ control, name: 'password' }}
            type="password"
            required
          />
          <FormInput
            outerLabel="User e-mail"
            controller={{ control, name: 'email' }}
            placeholder="User e-mail"
            required
          />
        </div>
        <div className="item">
          <FormInput
            outerLabel="First name"
            controller={{ control, name: 'first_name' }}
            placeholder="First name"
            required
          />
          <FormInput
            outerLabel="Last name"
            controller={{ control, name: 'last_name' }}
            placeholder="Last name"
            required
          />
          <FormInput
            controller={{ control, name: 'phone' }}
            outerLabel="Phone"
            placeholder="Phone"
            required
          />
        </div>
      </div>
      <div className="controls">
        <Button
          size={ButtonSize.BIG}
          text="Cancel"
          className="cancel"
          onClick={() => setModalState({ visible: false })}
          tabIndex={4}
          type="button"
        />
        <Button
          className="big primary"
          type="submit"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Add user"
          disabled={!isValid}
          loading={addUserMutation.isLoading}
        />
      </div>
    </form>
  );
};

export default AddUserForm;
