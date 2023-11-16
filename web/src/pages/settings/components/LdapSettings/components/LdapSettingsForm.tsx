import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { SettingsLDAP } from '../../../../../shared/types';
import { useSettingsPage } from '../../../hooks/useSettingsPage';

type FormFields = SettingsLDAP;

export const LdapSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.ldapSettings;
  const submitRef = useRef<HTMLInputElement | null>(null);
  const settings = useSettingsPage((state) => state.settings);
  const {
    settings: { patchSettings },
  } = useApi();

  const queryClient = useQueryClient();

  const toaster = useToaster();

  const { isLoading, mutate } = useMutation({
    mutationFn: patchSettings,
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
  });

  const schema = useMemo(
    () =>
      z.object({
        ldap_url: z
          .string()
          .url(LL.form.error.invalid())
          .min(1, LL.form.error.required()),
        ldap_bind_username: z.string().min(1, LL.form.error.required()),
        ldap_bind_password: z.string().min(0, LL.form.error.required()),
        ldap_group_member_attr: z.string().min(1, LL.form.error.required()),
        ldap_group_obj_class: z.string().min(1, LL.form.error.required()),
        ldap_group_search_base: z.string().min(1, LL.form.error.required()),
        ldap_groupname_attr: z.string().min(1, LL.form.error.required()),
        ldap_member_attr: z.string().min(1, LL.form.error.required()),
        ldap_user_obj_class: z.string().min(1, LL.form.error.required()),
        ldap_user_search_base: z.string().min(1, LL.form.error.required()),
        ldap_username_attr: z.string().min(1, LL.form.error.required()),
      }),
    [LL.form.error],
  );

  const defaultValues = useMemo(
    (): FormFields => ({
      ldap_group_search_base: settings?.ldap_group_search_base ?? '',
      ldap_group_member_attr: settings?.ldap_group_member_attr ?? '',
      ldap_group_obj_class: settings?.ldap_group_obj_class ?? '',
      ldap_username_attr: settings?.ldap_username_attr ?? '',
      ldap_user_search_base: settings?.ldap_user_search_base ?? '',
      ldap_user_obj_class: settings?.ldap_user_obj_class ?? '',
      ldap_url: settings?.ldap_url ?? '',
      ldap_member_attr: settings?.ldap_member_attr ?? '',
      ldap_groupname_attr: settings?.ldap_groupname_attr ?? '',
      ldap_bind_password: settings?.ldap_bind_password ?? '',
      ldap_bind_username: settings?.ldap_bind_username ?? '',
    }),
    [settings],
  );

  const { handleSubmit, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };
  return (
    <section id="ldap-settings">
      <header>
        <h2>{localLL.title()}</h2>
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text={LL.common.controls.saveChanges()}
          type="submit"
          loading={isLoading}
          icon={<IconCheckmarkWhite />}
          onClick={() => submitRef.current?.click()}
        />
      </header>
      <form id="ldap-settings-form" onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'ldap_url' }}
          label={localLL.form.labels.ldap_url()}
        />
        <FormInput
          controller={{ control, name: 'ldap_bind_username' }}
          label={localLL.form.labels.ldap_bind_username()}
        />
        <FormInput
          controller={{ control, name: 'ldap_bind_password' }}
          label={localLL.form.labels.ldap_bind_password()}
        />
        <FormInput
          controller={{ control, name: 'ldap_member_attr' }}
          label={localLL.form.labels.ldap_member_attr()}
        />
        <FormInput
          controller={{ control, name: 'ldap_username_attr' }}
          label={localLL.form.labels.ldap_username_attr()}
        />
        <FormInput
          controller={{ control, name: 'ldap_user_search_base' }}
          label={localLL.form.labels.ldap_user_search_base()}
        />
        <FormInput
          controller={{ control, name: 'ldap_user_obj_class' }}
          label={localLL.form.labels.ldap_user_obj_class()}
        />
        <FormInput
          controller={{ control, name: 'ldap_groupname_attr' }}
          label={localLL.form.labels.ldap_groupname_attr()}
        />
        <FormInput
          controller={{ control, name: 'ldap_group_obj_class' }}
          label={localLL.form.labels.ldap_group_obj_class()}
        />
        <FormInput
          controller={{ control, name: 'ldap_group_member_attr' }}
          label={localLL.form.labels.ldap_group_member_attr()}
        />
        <FormInput
          controller={{ control, name: 'ldap_group_search_base' }}
          label={localLL.form.labels.ldap_group_search_base()}
        />
        <input type="submit" aria-hidden="true" className="hidden" ref={submitRef} />
      </form>
    </section>
  );
};
