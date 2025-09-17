import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { omit } from 'lodash-es';
import { useMemo, useRef, useState } from 'react';
import { type SubmitHandler, useController, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { useAppStore } from '../../../../../../../shared/hooks/store/useAppStore';
import { useEnterpriseUpgradeStore } from '../../../../../../../shared/hooks/store/useEnterpriseUpgradeStore';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import {
  patternSafeUsernameCharacters,
  patternValidPhoneNumber,
} from '../../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../../shared/queries';
import { invalidateMultipleQueries } from '../../../../../../../shared/utils/invalidateMultipleQueries';
import { trimObjectStrings } from '../../../../../../../shared/utils/trimObjectStrings';
import { passwordValidator } from '../../../../../../../shared/validators/password';
import { useAddUserModal } from '../../hooks/useAddUserModal';
import { removeEmptyStrings } from '../../../../../../../shared/utils/removeEmptyStrings';

export const AddUserForm = () => {
  const { LL } = useI18nContext();
  const {
    user: { addUser, usernameAvailable, getUsers },
    getAppInfo,
  } = useApi();

  const reservedUserNames = useRef<string[]>([]);

  const { data: userEmails, isLoading: emailsLoading } = useQuery({
    queryKey: ['user'],
    queryFn: getUsers,
    refetchOnWindowFocus: false,
    refetchOnMount: true,
    select: (users) => users.map((user) => user.email),
    placeholderData: (perv) => perv,
  });

  const [checkingUsername, setCheckingUsername] = useState(false);

  const zodSchema = useMemo(
    () =>
      z
        .object({
          username: z
            .string()
            .min(1, LL.form.error.minimumLength())
            .max(64, LL.form.error.maximumLength())
            .regex(patternSafeUsernameCharacters, LL.form.error.forbiddenCharacter()),
          // check in refine
          password: z.string(),
          email: z
            .string()
            .trim()
            .min(1, LL.form.error.required())
            .email(LL.form.error.invalid())
            .refine((value) => {
              if (isPresent(userEmails)) {
                return !userEmails.includes(value.toLowerCase());
              }
              return true;
            }, LL.modals.addUser.form.error.emailReserved()),
          last_name: z.string().min(1, LL.form.error.required()),
          first_name: z.string().min(1, LL.form.error.required()),
          phone: z.string(),
          enable_enrollment: z.boolean(),
        })
        .superRefine((val, ctx) => {
          // check password
          if (!val.enable_enrollment) {
            const passResult = passwordValidator(LL).safeParse(val.password);
            if (!passResult.success) {
              passResult.error.issues.forEach((i) => {
                ctx.addIssue({
                  path: ['password'],
                  code: 'custom',
                  message: i.message,
                });
              });
            }
          }
          if (val.phone?.length) {
            const phoneRes = z
              .string()
              .regex(patternValidPhoneNumber)
              .safeParse(val.phone);
            if (!phoneRes.success) {
              ctx.addIssue({
                code: 'custom',
                path: ['phone'],
                message: LL.form.error.invalid(),
              });
            }
          }
          if (reservedUserNames.current.includes(val.username)) {
            ctx.addIssue({
              code: 'custom',
              path: ['username'],
              message: LL.form.error.usernameTaken(),
            });
          }
        }),
    [LL, userEmails],
  );

  type FormFields = z.infer<typeof zodSchema>;

  const {
    handleSubmit,
    control,
    formState: { isValid },
    trigger,
  } = useForm<FormFields>({
    resolver: zodResolver(zodSchema),
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

  const setAppStore = useAppStore((s) => s.setState, shallow);

  const showUpgradeToast = useEnterpriseUpgradeStore((s) => s.show);

  const toaster = useToaster();

  const [setModalState, nextStep, close] = useAddUserModal(
    (state) => [state.setState, state.nextStep, state.close],
    shallow,
  );

  const addUserMutation = useMutation({
    mutationFn: addUser,
    onSuccess: (user) => {
      // check license limits
      void getAppInfo().then((response) => {
        setAppStore({
          appInfo: response,
        });
        if (response.license_info.any_limit_exceeded) {
          showUpgradeToast();
        }
      });

      invalidateMultipleQueries(queryClient, [QueryKeys.FETCH_USERS_LIST]);

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
    onError: () => {
      toaster.error(LL.messages.error());
    },
  });

  const onSubmit: SubmitHandler<FormFields> = (data) => {
    const clean = removeEmptyStrings(trimObjectStrings(data));
    if (reservedUserNames.current.includes(clean.username)) {
      void trigger('username', { shouldFocus: true });
    } else {
      usernameAvailable(clean.username)
        .then(() => {
          setCheckingUsername(false);
          if (clean.enable_enrollment) {
            const userData = omit(clean, ['password', 'enable_enrollment']);
            addUserMutation.mutate(userData);
          } else {
            addUserMutation.mutate(omit(clean, ['enable_enrollment']));
          }
        })
        .catch(() => {
          setCheckingUsername(false);
          reservedUserNames.current = [...reservedUserNames.current, clean.username];
          void trigger('username', { shouldFocus: true });
        });
    }
  };

  return (
    <form
      id="add-user-form"
      data-testid="add-user-form"
      onSubmit={handleSubmit(onSubmit)}
    >
      <div className="checkbox-space">
        <FormCheckBox
          labelPlacement="right"
          label={LL.modals.addUser.form.fields.enableEnrollment.label()}
          controller={{ control, name: 'enable_enrollment' }}
        />
        {parse(LL.modals.addUser.form.fields.enableEnrollment.link())}
      </div>
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
          disabled={addUserMutation.isPending || checkingUsername}
        />
        <Button
          className="big primary"
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.modals.addUser.form.submit()}
          disabled={!isValid}
          loading={addUserMutation.isPending || checkingUsername || emailsLoading}
        />
      </div>
    </form>
  );
};
