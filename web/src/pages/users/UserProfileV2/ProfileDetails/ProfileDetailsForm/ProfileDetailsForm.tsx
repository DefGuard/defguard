import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { pick } from 'lodash-es';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { useLocation, useNavigate } from 'react-router';
import * as yup from 'yup';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/components/Form/FormSelect/FormSelect';
import { SelectOption } from '../../../../../shared/components/layout/Select/Select';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useUserProfileV2Store } from '../../../../../shared/hooks/store/useUserProfileV2Store';
import useApi from '../../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../../shared/mutations';
import {
  patternNoSpecialChars,
  patternValidEmail,
  patternValidPhoneNumber,
} from '../../../../../shared/patterns';
import { QueryKeys } from '../../../../../shared/queries';
import { omitNull } from '../../../../../shared/utils/omitNull';
import { titleCase } from '../../../../../shared/utils/titleCase';
import { toaster } from '../../../../../shared/utils/toaster';

interface Inputs {
  username: string;
  first_name: string;
  last_name: string;
  phone: string;
  email: string;
  groups: SelectOption<string>[];
}

const defaultValues: Inputs = {
  username: '',
  first_name: '',
  last_name: '',
  phone: '',
  email: '',
  groups: [],
};

export const ProfileDetailsForm = () => {
  const { t } = useTranslation('en');

  const user = useUserProfileV2Store((state) => state.user);
  const submitSubject = useUserProfileV2Store((state) => state.submitSubject);
  const setUserProfile = useUserProfileV2Store((state) => state.setState);
  const submitButton = useRef<HTMLButtonElement | null>(null);
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const isAdmin = useAuthStore((state) => state.isAdmin);
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
            .required(t('form.errors.required'))
            .matches(patternNoSpecialChars, t('form.errors.noSpecialChars'))
            .max(32, t('form.errors.maximumLength', { length: 32 })),
          first_name: yup
            .string()
            .required(t('form.errors.required'))
            .min(4, t('form.errors.minimumLength', { length: 4 })),
          last_name: yup
            .string()
            .required(t('form.errors.required'))
            .min(4, t('form.errors.minimumLength', { length: 4 })),
          phone: yup
            .string()
            .required(t('form.errors.required'))
            .matches(patternValidPhoneNumber, t('form.errors.phoneNumber')),
          email: yup
            .string()
            .required(t('form.errors.required'))
            .matches(patternValidEmail, t('form.errors.email')),
          groups: yup.array(),
        })
        .required(),
    [t]
  );

  const formDefaultValues = useMemo(() => {
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
    return res;
  }, [user]);

  const {
    control,
    handleSubmit,
    formState: { isValid },
  } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: formDefaultValues as Inputs,
  });

  const { data: availableGroups, isLoading: groupsLoading } = useQuery(
    [QueryKeys.FETCH_GROUPS],
    getGroups
  );
  const { mutate, isLoading: userEditLoading } = useMutation(
    [MutationKeys.EDIT_USER],
    editUser,
    {
      onSuccess: (response) => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USERS]);
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        toaster.success(`User ${response.username} modified.`);
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
      mutate({ username: user.username, data: { ...values, groups: groups } });
    }
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
    setUserProfile({ profileDetailsFormValid: isValid });
  }, [isValid, setUserProfile]);

  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <div className="row">
        <div className="item">
          <FormInput
            outerLabel="Username"
            controller={{ control, name: 'username' }}
            disabled={userEditLoading || !isAdmin}
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormInput
            outerLabel="First name"
            controller={{ control, name: 'first_name' }}
            disabled={userEditLoading || !isAdmin}
          />
        </div>
        <div className="item">
          <FormInput
            outerLabel="Last name"
            controller={{ control, name: 'last_name' }}
            disabled={userEditLoading || !isAdmin}
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
          />
        </div>
      </div>
      <div className="row">
        <div className="item">
          <FormSelect
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
