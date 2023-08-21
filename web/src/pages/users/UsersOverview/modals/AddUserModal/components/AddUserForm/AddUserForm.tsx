import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { omit } from 'lodash-es';
import { useMemo, useRef, useState } from 'react';
import { SubmitHandler, useController, useForm } from 'react-hook-form';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import {
  patternDigitOrLowercase,
  patternNoSpecialChars,
  patternStartsWithDigit,
  patternValidEmail,
  patternValidPhoneNumber,
} from '../../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../../shared/queries';
import { passwordValidator } from '../../../../../../../shared/validators/password';
import { useAddUserModal } from '../../hooks/useAddUserModal';

interface Inputs {
  username: string;
  email: string;
  last_name: string;
  first_name: string;
  enable_enrollment: boolean;
  // disabled when enableEnrollment is true
  password?: string;
  phone?: string;
}

export const AddUserForm = () => {
  const { LL } = useI18nContext();
  const {
    user: { addUser, usernameAvailable },
  } = useApi();

  const reservedUserNames = useRef<string[]>([]);

  const [checkingUsername, setCheckingUsername] = useState(false);

  const formSchema = useMemo(
    () =>
      yup
        .object({
          username: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternNoSpecialChars, LL.form.error.noSpecialChars())
            .matches(patternDigitOrLowercase, LL.form.error.invalid())
            .min(3, LL.form.error.minimumLength())
            .max(64, LL.form.error.maximumLength())
            .test('starts-with-number', LL.form.error.startFromNumber(), (value) => {
              if (value && value.length) {
                return !patternStartsWithDigit.test(value);
              }
              return false;
            })
            .test(
              'username-available',
              LL.form.error.usernameTaken(),
              (value?: string) =>
                value ? !reservedUserNames.current.includes(value) : false,
            ),
          password: yup
            .string()
            .when('enable_enrollment', { is: false, then: () => passwordValidator(LL) }),
          email: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternValidEmail, LL.form.error.invalid()),
          last_name: yup.string().required(LL.form.error.required()),
          first_name: yup.string().required(LL.form.error.required()),
          phone: yup
            .string()
            .optional()
            .test('is-valid', LL.form.error.invalid(), (value) => {
              if (value && value.length) {
                return patternValidPhoneNumber.test(value);
              }
              return true;
            }),
          enable_enrollment: yup.boolean(),
        })
        .required(),
    [LL],
  );

  const {
    handleSubmit,
    control,
    formState: { isValid },
    trigger,
  } = useForm<Inputs>({
    resolver: yupResolver(formSchema),
    mode: 'all',
    criteriaMode: 'all',
    defaultValues: {
      email: '',
      first_name: '',
      last_name: '',
      password: '',
      phone: '',
      username: '',
      enable_enrollment: false,
    },
  });

  const {
    field: { value: enableEnrollment },
  } = useController({ control, name: 'enable_enrollment' });

  const queryClient = useQueryClient();

  const toaster = useToaster();

  const [setModalState, nextStep, close] = useAddUserModal(
    (state) => [state.setState, state.nextStep, state.close],
    shallow,
  );

  const addUserMutation = useMutation(addUser, {
    onSuccess: (user) => {
      queryClient.invalidateQueries([QueryKeys.FETCH_USERS_LIST]);
      if (enableEnrollment) {
        toaster.success(LL.modals.addUser.messages.userAdded());
        setModalState({
          user: user,
        });
        nextStep();
      } else {
        close();
      }
    },
    onError: (err) => {
      close();
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const onSubmit: SubmitHandler<Inputs> = async (data) => {
    if (reservedUserNames.current.includes(data.username)) {
      trigger('username', { shouldFocus: true });
    } else {
      usernameAvailable(data.username)
        .then(() => {
          setCheckingUsername(false);
          if (data.enable_enrollment) {
            const userData = omit(data, ['password', 'enable_enrollment']);
            addUserMutation.mutate(userData);
          } else {
            if (data.password) {
              addUserMutation.mutate(omit(data, ['enable_enrollment']));
            } else {
              trigger('password', { shouldFocus: true });
            }
          }
        })
        .catch(() => {
          setCheckingUsername(false);
          reservedUserNames.current = [...reservedUserNames.current, data.username];
          trigger('username', { shouldFocus: true });
        });
    }
  };

  return (
    <form
      id="add-user-form"
      data-testid="add-user-form"
      onSubmit={handleSubmit(onSubmit)}
    >
      <FormCheckBox
        labelPlacement="right"
        label={LL.modals.addUser.form.fields.enableEnrollment.label()}
        controller={{ control, name: 'enable_enrollment' }}
      />
      <div className="row">
        <div className="item">
          <FormInput
            placeholder={LL.modals.addUser.form.fields.username.placeholder()}
            controller={{ control, name: 'username' }}
            label={LL.modals.addUser.form.fields.username.label()}
            autoComplete="username"
            required
          />
          <FormInput
            label={LL.modals.addUser.form.fields.password.label()}
            placeholder={LL.modals.addUser.form.fields.password.placeholder()}
            controller={{ control, name: 'password' }}
            floatingErrors={{
              title: LL.form.floatingErrors.title(),
            }}
            type="password"
            autoComplete="password"
            required={!enableEnrollment}
            disabled={enableEnrollment}
          />
          <FormInput
            label={LL.modals.addUser.form.fields.email.label()}
            placeholder={LL.modals.addUser.form.fields.email.placeholder()}
            controller={{ control, name: 'email' }}
            autoComplete="email"
            required
          />
        </div>
        <div className="item">
          <FormInput
            label={LL.modals.addUser.form.fields.firstName.label()}
            controller={{ control, name: 'first_name' }}
            placeholder={LL.modals.addUser.form.fields.firstName.placeholder()}
            autoComplete="given-name"
            required
          />
          <FormInput
            label={LL.modals.addUser.form.fields.lastName.label()}
            controller={{ control, name: 'last_name' }}
            placeholder={LL.modals.addUser.form.fields.lastName.placeholder()}
            autoComplete="family-name"
            required
          />
          <FormInput
            controller={{ control, name: 'phone' }}
            label={LL.modals.addUser.form.fields.phone.label()}
            placeholder={LL.modals.addUser.form.fields.phone.placeholder()}
            autoComplete="tel"
          />
        </div>
      </div>
      <div className="controls">
        <Button
          size={ButtonSize.LARGE}
          text={LL.form.cancel()}
          className="cancel"
          onClick={() => setModalState({ visible: false })}
          tabIndex={4}
          type="button"
          disabled={addUserMutation.isLoading || checkingUsername}
        />
        <Button
          className="big primary"
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.modals.addUser.form.submit()}
          disabled={!isValid}
          loading={addUserMutation.isLoading || checkingUsername}
        />
      </div>
    </form>
  );
};
