import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { pick, values } from 'lodash-es';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Controller, SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate, useParams } from 'react-router';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../../../shared/hooks/store/useAuthStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import {
  patternSafeUsernameCharacters,
  patternValidEmail,
  patternValidPhoneNumber,
} from '../../../../../shared/patterns';
import { QueryKeys } from '../../../../../shared/queries';
import { OAuth2AuthorizedApps } from '../../../../../shared/types';
import { omitNull } from '../../../../../shared/utils/omitNull';
import { titleCase } from '../../../../../shared/utils/titleCase';
import { trimObjectStrings } from '../../../../../shared/utils/trimObjectStrings';
import { ProfileDetailsFormAppsField } from './ProfileDetailsFormAppsField';

interface Inputs {
  username: string;
  first_name: string;
  last_name: string;
  phone: string;
  email: string;
  groups: string[];
  authorized_apps: OAuth2AuthorizedApps[];
  is_active: boolean;
}

const defaultValues: Inputs = {
  username: '',
  first_name: '',
  last_name: '',
  phone: '',
  email: '',
  groups: [],
  authorized_apps: [],
  is_active: true,
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
  const isMe = useUserProfileStore((state) => state.isMe);
  const [fetchGroups, setFetchGroups] = useState(false);
  const {
    user: { editUser },
    groups: { getGroups },
  } = useApi();
  const { username: paramsUsername } = useParams();
  const navigate = useNavigate();
  const [usernameChangeWarning, setUsernameChangeWarning] = useState(false);

  const zodSchema = useMemo(
    () =>
      z.object({
        username: z
          .string()
          .min(1, LL.form.error.required())
          .regex(patternSafeUsernameCharacters, LL.form.error.forbiddenCharacter())
          .min(3, LL.form.error.minimumLength())
          .max(64, LL.form.error.maximumLength()),
        first_name: z.string().min(1, LL.form.error.required()),
        last_name: z.string().min(1, LL.form.error.required()),
        phone: z
          .string()
          .optional()
          .refine((val) => {
            if (val && values.length > 0) {
              return patternValidPhoneNumber.test(val);
            }
            return true;
          }, LL.form.error.invalid()),
        email: z
          .string()
          .min(1, LL.form.error.required())
          .regex(patternValidEmail, LL.form.error.invalid()),
        groups: z.array(z.string().min(1, LL.form.error.required())),
        authorized_apps: z.array(
          z.object({
            oauth2client_id: z.number().min(1, LL.form.error.required()),
            oauth2client_name: z.string().min(1, LL.form.error.required()),
            user_id: z.number().min(1, LL.form.error.required()),
          }),
        ),
        is_active: z.boolean(),
      }),
    [LL.form.error],
  );

  const formDefaultValues = useMemo((): Inputs => {
    const omitted = pick(omitNull(userProfile?.user), Object.keys(defaultValues));
    const res = { ...defaultValues, ...omitted };
    return res as Inputs;
  }, [userProfile]);

  const { control, handleSubmit, setValue, getValues } = useForm<Inputs>({
    resolver: zodResolver(zodSchema),
    mode: 'all',
    defaultValues: formDefaultValues,
  });

  const { data: availableGroups, isLoading: groupsLoading } = useQuery(
    [QueryKeys.FETCH_GROUPS],
    getGroups,
    {
      refetchOnWindowFocus: false,
      enabled: fetchGroups && isAdmin,
    },
  );
  const toaster = useToaster();
  const { mutate, isLoading: userEditLoading } = useMutation(
    [MutationKeys.EDIT_USER],
    editUser,
    {
      onSuccess: (_data, variables) => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USERS_LIST]);
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        toaster.success(LL.userPage.messages.editSuccess());
        setUserProfile({ editMode: false, loading: false });
        // if username was changed redirect to new profile page
        const newUsername = variables.data.username;
        if (paramsUsername !== newUsername) {
          navigate(`/admin/users/${variables.data.username}`, { replace: true });
        }
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

  const statusOptions = useMemo(() => {
    return [
      {
        key: 'active',
        value: true,
        label: LL.userPage.userDetails.fields.status.active(),
      },
      {
        key: 'inactive',
        value: false,
        label: LL.userPage.userDetails.fields.status.disabled(),
      },
    ];
  }, [LL.userPage.userDetails.fields.status]);

  const onValidSubmit: SubmitHandler<Inputs> = (values) => {
    values = trimObjectStrings(values);
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
        if (getValues().username !== userProfile?.user.username) {
          setUsernameChangeWarning(true);
          return;
        }
        submitButton.current?.click();
      });
      return () => sub.unsubscribe();
    }
  }, [submitSubject, getValues, userProfile?.user.username]);

  useEffect(() => {
    setTimeout(() => setFetchGroups(true), 500);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <>
      <form onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}>
        <ModalWithTitle
          className="change-warning-modal"
          backdrop
          isOpen={usernameChangeWarning}
          onClose={() => {
            setUsernameChangeWarning(false);
          }}
          title="Warning"
        >
          <p>{LL.userPage.userDetails.warningModals.content.usernameChange()}</p>
          <div className="buttons">
            <Button
              text={LL.userPage.userDetails.warningModals.buttons.proceed()}
              styleVariant={ButtonStyleVariant.DELETE}
              onClick={() => {
                setUsernameChangeWarning(false);
                submitButton.current?.click();
              }}
            />
            <Button
              onClick={() => {
                setUsernameChangeWarning(false);
              }}
              text={LL.userPage.userDetails.warningModals.buttons.cancel()}
            />
          </div>
        </ModalWithTitle>
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
        {isAdmin && !isMe && (
          <div className="row">
            <div className="item">
              <FormSelect
                data-testid="status-select"
                options={statusOptions}
                controller={{ control, name: 'is_active' }}
                label={LL.userPage.userDetails.fields.status.label()}
                disabled={userEditLoading || !isAdmin}
                renderSelected={(val) => ({
                  key: val ? 'active' : 'inactive',
                  displayValue: val
                    ? LL.userPage.userDetails.fields.status.active()
                    : LL.userPage.userDetails.fields.status.disabled(),
                })}
              />
            </div>
          </div>
        )}
        <div className="row">
          <div className="item">
            <FormSelect
              data-testid="groups-select"
              options={groupsOptions}
              controller={{ control, name: 'groups' }}
              label={LL.userPage.userDetails.fields.groups.label()}
              loading={isAdmin && (groupsLoading || userEditLoading)}
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
    </>
  );
};
