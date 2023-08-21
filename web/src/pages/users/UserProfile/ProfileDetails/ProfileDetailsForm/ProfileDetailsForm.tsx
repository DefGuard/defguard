import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { pick } from 'lodash-es';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Controller, SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import {
  patternNoSpecialChars,
  patternStartsWithDigit,
  patternValidEmail,
  patternValidPhoneNumber,
} from '../../../../../shared/patterns';
import { QueryKeys } from '../../../../../shared/queries';
import { OAuth2AuthorizedApps } from '../../../../../shared/types';
import { omitNull } from '../../../../../shared/utils/omitNull';
import { titleCase } from '../../../../../shared/utils/titleCase';
import { ProfileDetailsFormAppsField } from './ProfileDetailsFormAppsField';

interface Inputs {
  username: string;
  first_name: string;
  last_name: string;
  phone: string;
  email: string;
  groups: string[];
  authorized_apps: OAuth2AuthorizedApps[];
}

const defaultValues: Inputs = {
  username: '',
  first_name: '',
  last_name: '',
  phone: '',
  email: '',
  groups: [],
  authorized_apps: [],
};

export const ProfileDetailsForm = () => {
  const { LL } = useI18nContext();
  const appSettings = useAppStore((state) => state.settings);
  const userProfile = useUserProfileStore((state) => state.userProfile);
  const submitSubject = useUserProfileStore((state) => state.submitSubject);
  const setUserProfile = useUserProfileStore((state) => state.setState);
  const submitButton = useRef<HTMLButtonElement | null>(null);
  const queryClient = useQueryClient();
  const isAdmin = useAuthStore((state) => state.isAdmin);
  const [fetchGroups, setFetchGroups] = useState(false);
  const {
    user: { editUser },
    groups: { getGroups },
  } = useApi();

  const schema = useMemo(
    () =>
      yup
        .object({
          username: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternNoSpecialChars, LL.form.error.noSpecialChars())
            .min(3, LL.form.error.minimumLength())
            .max(64, LL.form.error.maximumLength())
            .test('starts-with-number', LL.form.error.startFromNumber(), (value) => {
              if (value && value.length) {
                return !patternStartsWithDigit.test(value);
              }
              return false;
            }),
          first_name: yup.string().required(LL.form.error.required()),
          last_name: yup.string().required(LL.form.error.required()),
          phone: yup
            .string()
            .optional()
            .test('is-valid', LL.form.error.invalid(), (value) => {
              if (value && value.length) {
                return patternValidPhoneNumber.test(value);
              }
              return true;
            }),
          email: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternValidEmail, LL.form.error.invalid()),
          groups: yup.array(),
          authorized_apps: yup.array().of(
            yup.object().shape({
              oauth2client_id: yup.number().required(),
              oauth2client_name: yup.string().required(),
              user_id: yup.number().required(),
            }),
          ),
        })
        .required(),
    [LL.form.error],
  );

  const formDefaultValues = useMemo((): Inputs => {
    const ommited = pick(omitNull(userProfile?.user), Object.keys(defaultValues));
    const res = { ...defaultValues, ...ommited };
    return res as Inputs;
  }, [userProfile]);

  const { control, handleSubmit, setValue, getValues } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: formDefaultValues,
  });

  const { data: availableGroups, isLoading: groupsLoading } = useQuery(
    [QueryKeys.FETCH_GROUPS],
    getGroups,
    {
      refetchOnWindowFocus: false,
      enabled: fetchGroups,
    },
  );
  const toaster = useToaster();
  const { mutate, isLoading: userEditLoading } = useMutation(
    [MutationKeys.EDIT_USER],
    editUser,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USERS_LIST]);
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        toaster.success(LL.userPage.messages.editSuccess());
        setUserProfile({ editMode: false, loading: false });
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setUserProfile({ loading: false });
        console.error(err);
      },
    },
  );

  const groupsOptions = useMemo(() => {
    if (availableGroups && !groupsLoading) {
      return availableGroups.groups?.map((g) => ({
        key: g,
        value: g,
        label: titleCase(g),
      }));
    }
    return [];
  }, [availableGroups, groupsLoading]);

  const onValidSubmit: SubmitHandler<Inputs> = (values) => {
    if (userProfile && userProfile.user) {
      setUserProfile({ loading: true });
      mutate({
        username: userProfile.user.username,
        data: {
          ...userProfile.user,
          ...values,
          totp_enabled: userProfile.user.totp_enabled,
        },
      });
    }
  };

  // When submitted errors will be visible.
  const onInvalidSubmit: SubmitErrorHandler<Inputs> = (values) => {
    const invalidFields = Object.keys(values) as (keyof Partial<Inputs>)[];
    const invalidFieldsValues = getValues(invalidFields);
    invalidFields.forEach((key, index) => {
      setValue(key, invalidFieldsValues[index], {
        shouldTouch: true,
        shouldValidate: true,
      });
    });
  };

  useEffect(() => {
    if (submitButton && submitButton.current) {
      const sub = submitSubject.subscribe(() => {
        submitButton.current?.click();
      });
      return () => sub.unsubscribe();
    }
  }, [submitSubject]);

  useEffect(() => {
    setTimeout(() => setFetchGroups(true), 500);
  }, []);

  return (
    <form onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}>
      <div className="row">
        <div className="item">
          <FormInput
            label={LL.userPage.userDetails.fields.username.label()}
            controller={{ control, name: 'username' }}
            disabled={userEditLoading || !isAdmin}
            required
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormInput
            label={LL.userPage.userDetails.fields.firstName.label()}
            controller={{ control, name: 'first_name' }}
            disabled={userEditLoading || !isAdmin}
            required
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormInput
            label={LL.userPage.userDetails.fields.lastName.label()}
            controller={{ control, name: 'last_name' }}
            disabled={userEditLoading || !isAdmin}
            required
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormInput
            label={LL.userPage.userDetails.fields.phone.label()}
            controller={{ control, name: 'phone' }}
            disabled={userEditLoading}
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormInput
            label={LL.userPage.userDetails.fields.email.label()}
            controller={{ control, name: 'email' }}
            disabled={userEditLoading || !isAdmin}
            required
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormSelect
            data-testid="groups-select"
            options={groupsOptions}
            controller={{ control, name: 'groups' }}
            label={LL.userPage.userDetails.fields.groups.label()}
            loading={groupsLoading || userEditLoading}
            disabled={!isAdmin}
            renderSelected={(val) => ({
              key: val,
              displayValue: titleCase(val),
            })}
          />
        </div>
      </div>
      {appSettings?.openid_enabled && (
        <div className="row tags">
          <Controller
            control={control}
            name="authorized_apps"
            render={({ field }) => (
              <ProfileDetailsFormAppsField
                value={field.value}
                onChange={field.onChange}
              />
            )}
          />
        </div>
      )}
      <button type="submit" className="hidden" ref={submitButton} />
    </form>
  );
};
