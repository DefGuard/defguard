import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { pick } from 'lodash-es';
import { useEffect, useMemo, useRef, useState } from 'react';
import { SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import { useLocation, useNavigate } from 'react-router';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/components/Form/FormSelect/FormSelect';
import {
  SelectOption,
  SelectStyleVariant,
} from '../../../../../shared/components/layout/Select/Select';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import {
  patternNoSpecialChars,
  patternValidEmail,
  patternValidPhoneNumber,
} from '../../../../../shared/patterns';
import { QueryKeys } from '../../../../../shared/queries';
import { OAuth2AuthorizedApps } from '../../../../../shared/types';
import { omitNull } from '../../../../../shared/utils/omitNull';
import { titleCase } from '../../../../../shared/utils/titleCase';

interface Inputs {
  username: string;
  first_name: string;
  last_name: string;
  phone: string;
  email: string;
  groups: SelectOption<string>[];
  oauth_tokens: SelectOption<OAuth2AuthorizedApps>[];
}

const defaultValues: Inputs = {
  username: '',
  first_name: '',
  last_name: '',
  phone: '',
  email: '',
  groups: [],
  oauth_tokens: [],
};

export const ProfileDetailsForm = () => {
  const { LL, locale } = useI18nContext();
  const user = useUserProfileStore((state) => state.user);
  const submitSubject = useUserProfileStore((state) => state.submitSubject);
  const setUserProfile = useUserProfileStore((state) => state.setState);
  const submitButton = useRef<HTMLButtonElement | null>(null);
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
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
            .max(32, LL.form.error.maximumLength()),
          first_name: yup
            .string()
            .required(LL.form.error.required())
            .min(4, LL.form.error.minimumLength()),
          last_name: yup
            .string()
            .required(LL.form.error.required())
            .min(4, LL.form.error.minimumLength()),
          phone: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternValidPhoneNumber, LL.form.error.invalid()),
          email: yup
            .string()
            .required(LL.form.error.required())
            .matches(patternValidEmail, LL.form.error.invalid()),
          groups: yup.array(),
          authorized_apps: yup.array(),
        })
        .required(),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [locale]
  );

  const formDefaultValues = useMemo((): Inputs => {
    const ommited = pick(omitNull(user), Object.keys(defaultValues));
    const res = { ...defaultValues, ...ommited };
    if (ommited.groups) {
      const groupOptions: SelectOption<string>[] = ommited.groups.map((g) => ({
        key: g,
        value: g,
        label: titleCase(g),
      }));
      res.groups = groupOptions;
    } else {
      res.groups = [];
    }
    if (ommited.authorized_apps) {
      const appsOptions: SelectOption<OAuth2AuthorizedApps>[] =
        ommited.authorized_apps.map((a) => ({
          key: a.oauth2client_id,
          value: a,
          label: a.oauth2client_name,
        }));
      res.oauth_tokens = appsOptions;
    } else {
      res.oauth_tokens = [];
    }
    return res as Inputs;
  }, [user]);

  const { control, handleSubmit, setValue, getValues } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: formDefaultValues as Inputs,
  });

  const { data: availableGroups, isLoading: groupsLoading } = useQuery(
    [QueryKeys.FETCH_GROUPS],
    getGroups,
    {
      refetchOnWindowFocus: false,
      enabled: fetchGroups,
    }
  );
  const toaster = useToaster();
  const { mutate, isLoading: userEditLoading } = useMutation(
    [MutationKeys.EDIT_USER],
    editUser,
    {
      onSuccess: (_, request) => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USERS]);
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        toaster.success(`User ${request.username} modified.`);
        setUserProfile({ editMode: false });
        if (location.pathname.includes('/edit')) {
          navigate('../');
        }
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Error occured!', 'Please contact administrator');
      },
    }
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
    if (user) {
      const groups = values.groups.map((g) => g.value);
      const apps = values.oauth_tokens.map((a) => a.value);
      mutate({
        username: user.username,
        data: {
          ...user,
          ...values,
          groups: groups,
          totp_enabled: user.totp_enabled,
          authorized_apps: apps,
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
            outerLabel="Username"
            controller={{ control, name: 'username' }}
            disabled={userEditLoading || !isAdmin}
            required
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormInput
            outerLabel="First name"
            controller={{ control, name: 'first_name' }}
            disabled={userEditLoading || !isAdmin}
            required
          />
        </div>
        <div className="item">
          <FormInput
            outerLabel="Last name"
            controller={{ control, name: 'last_name' }}
            disabled={userEditLoading || !isAdmin}
            required
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormInput
            outerLabel="Phone number"
            controller={{ control, name: 'phone' }}
            disabled={userEditLoading}
          />
        </div>
        <div className="item">
          <FormInput
            outerLabel="E-Mail"
            controller={{ control, name: 'email' }}
            disabled={userEditLoading || !isAdmin}
            required
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormSelect
            styleVariant={SelectStyleVariant.WHITE}
            options={groupsOptions}
            controller={{ control, name: 'groups' }}
            outerLabel="Groups"
            loading={groupsLoading || userEditLoading}
            searchable={true}
            multi={true}
            disabled={!isAdmin}
          />
        </div>
      </div>
      <button type="submit" className="hidden" ref={submitButton} />
    </form>
  );
};
