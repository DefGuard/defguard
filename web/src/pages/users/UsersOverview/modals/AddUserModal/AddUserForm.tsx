import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosResponse } from 'axios';
import { useEffect, useMemo, useState } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import { BehaviorSubject, Subject } from 'rxjs';
import {
  debounceTime,
  distinctUntilChanged,
  filter,
  map,
  switchMap,
} from 'rxjs/operators';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../i18n/i18n-react';
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

export const AddUserForm = () => {
  const { LL, locale } = useI18nContext();
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
            .required(LL.form.error.required())
            .matches(patternNoSpecialChars, LL.form.error.noSpecialChars())
            .matches(patternDigitOrLowercase, LL.form.error.invalid())
            .min(4, LL.form.error.minimumLength)
            .test('username-available', LL.form.error.usernameTaken(), (value?: string) =>
              value ? !usernamesTaken.getValue().includes(value) : false
            ),
          password: yup
            .string()
            .min(8, LL.form.error.minimumLength())
            .max(32, LL.form.error.maximumLength())
            .matches(patternAtLeastOneDigit, LL.form.error.invalid())
            .matches(patternAtLeastOneSpecialChar, LL.form.error.invalid())
            .matches(patternAtLeastOneUpperCaseChar, LL.form.error.invalid())
            .matches(patternAtLeastOneLowerCaseChar, LL.form.error.invalid())
            .required(),
          email: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternValidEmail, LL.form.error.invalid()),
          last_name: yup
            .string()
            .required(LL.form.error.required())
            .min(4, LL.form.error.minimumLength()),
          first_name: yup
            .string()
            .required(LL.form.error.required())
            .min(4, LL.form.error.minimumLength()),
          phone: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternValidPhoneNumber, LL.form.error.invalid()),
        })
        .required(),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [locale, usernamesTaken]
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
      toaster.error('Error occurred.');
    },
  });

  const onSubmit: SubmitHandler<Inputs> = (data) => addUserMutation.mutate(data);

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
            { message: LL.form.error.usernameTaken() },
            { shouldFocus: true }
          );
        }
      });
    return () => {
      subscription.unsubscribe();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [usernameSubject, locale]);

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <div className="row">
        <div className="item">
          <FormInput
            placeholder={LL.modals.addUser.form.fields.username.placeholder()}
            controller={{ control, name: 'username' }}
            outerLabel={LL.modals.addUser.form.fields.username.label()}
            required
          />
          <FormInput
            outerLabel={LL.modals.addUser.form.fields.password.label()}
            placeholder={LL.modals.addUser.form.fields.password.placeholder()}
            controller={{ control, name: 'password' }}
            type="password"
            required
          />
          <FormInput
            outerLabel={LL.modals.addUser.form.fields.email.label()}
            placeholder={LL.modals.addUser.form.fields.email.placeholder()}
            controller={{ control, name: 'email' }}
            required
          />
        </div>
        <div className="item">
          <FormInput
            outerLabel={LL.modals.addUser.form.fields.firstName.label()}
            controller={{ control, name: 'first_name' }}
            placeholder={LL.modals.addUser.form.fields.firstName.placeholder()}
            required
          />
          <FormInput
            outerLabel={LL.modals.addUser.form.fields.lastName.label()}
            controller={{ control, name: 'last_name' }}
            placeholder={LL.modals.addUser.form.fields.lastName.placeholder()}
            required
          />
          <FormInput
            controller={{ control, name: 'phone' }}
            outerLabel={LL.modals.addUser.form.fields.phone.label()}
            placeholder={LL.modals.addUser.form.fields.phone.placeholder()}
            required
          />
        </div>
      </div>
      <div className="controls">
        <Button
          size={ButtonSize.BIG}
          text={LL.form.cancel()}
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
          text={LL.modals.addUser.form.submit()}
          disabled={!isValid}
          loading={addUserMutation.isLoading}
        />
      </div>
    </form>
  );
};
