import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import SvgIconX from '../../../../../shared/components/svg/IconX';
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
import { LdapConnectionTest } from './LdapConnectionTest';
import { LdapSettingsLeft } from './LdapSettingsLeft';
import { LdapSettingsRight } from './LdapSettingsRight';

type FormFields = Omit<SettingsLDAP, 'ldap_user_auxiliary_obj_classes'> & {
  ldap_user_auxiliary_obj_classes: string;
};

export const LdapSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.ldapSettings;
  const settings = useSettingsPage((state) => state.settings);
  const {
    settings: { patchSettings },
  } = useApi();
  const queryClient = useQueryClient();
  const toaster = useToaster();

  const { isPending: isLoading, mutate } = useMutation({
    mutationFn: patchSettings,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_SETTINGS],
      });
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (error) => {
      toaster.error(LL.messages.error());
      console.error(error);
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
        ldap_user_auxiliary_obj_classes: z.string(),
        ldap_user_search_base: z.string().min(1, LL.form.error.required()),
        ldap_username_attr: z.string().min(1, LL.form.error.required()),
        ldap_enabled: z.boolean(),
        ldap_sync_enabled: z.boolean(),
        ldap_is_authoritative: z.boolean(),
        ldap_use_starttls: z.boolean(),
        ldap_tls_verify_cert: z.boolean(),
        ldap_sync_interval: z.number().default(300),
        ldap_uses_ad: z.boolean(),
        ldap_user_rdn_attr: z.string().optional(),
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
      ldap_user_auxiliary_obj_classes:
        settings?.ldap_user_auxiliary_obj_classes.join(', ') ?? '',
      ldap_url: settings?.ldap_url ?? '',
      ldap_member_attr: settings?.ldap_member_attr ?? '',
      ldap_groupname_attr: settings?.ldap_groupname_attr ?? '',
      ldap_bind_password: settings?.ldap_bind_password ?? '',
      ldap_bind_username: settings?.ldap_bind_username ?? '',
      ldap_enabled: settings?.ldap_enabled ?? false,
      ldap_sync_enabled: settings?.ldap_sync_enabled ?? false,
      ldap_is_authoritative: settings?.ldap_is_authoritative ?? false,
      ldap_use_starttls: settings?.ldap_use_starttls ?? false,
      ldap_tls_verify_cert: settings?.ldap_tls_verify_cert ?? true,
      ldap_sync_interval: settings?.ldap_sync_interval ?? 300,
      ldap_uses_ad: settings?.ldap_uses_ad ?? false,
      ldap_user_rdn_attr: settings?.ldap_user_rdn_attr ?? '',
    }),
    [settings],
  );

  const emptyValues: FormFields = useMemo(
    () => ({
      ldap_group_search_base: '',
      ldap_group_member_attr: '',
      ldap_group_obj_class: '',
      ldap_username_attr: '',
      ldap_user_search_base: '',
      ldap_user_obj_class: '',
      ldap_user_auxiliary_obj_classes: '',
      ldap_url: '',
      ldap_member_attr: '',
      ldap_groupname_attr: '',
      ldap_bind_password: '',
      ldap_bind_username: '',
      ldap_enabled: false,
      ldap_sync_enabled: false,
      ldap_is_authoritative: false,
      ldap_use_starttls: false,
      ldap_tls_verify_cert: true,
      ldap_sync_interval: 300,
      ldap_uses_ad: false,
      ldap_user_rdn_attr: '',
    }),
    [],
  );

  const { handleSubmit, reset, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    const formattedData = {
      ...data,
      ldap_user_auxiliary_obj_classes: data.ldap_user_auxiliary_obj_classes
        .split(',')
        .map((obj_class) => obj_class.trim())
        .filter((obj_class) => obj_class.length > 0),
    };
    mutate(formattedData);
  };

  const handleDeleteSubmit = useCallback(() => {
    mutate({
      ...emptyValues,
      ldap_user_auxiliary_obj_classes: [],
    });
    reset(emptyValues);
  }, [mutate, emptyValues, reset]);

  return (
    <section id="ldap-settings">
      <header>
        <h2>{localLL.title()}</h2>
        <div className="controls">
          <LdapConnectionTest />
          <Button
            text={localLL.form.delete()}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.CONFIRM}
            loading={isLoading}
            icon={<SvgIconX />}
            onClick={() => {
              handleDeleteSubmit();
            }}
          />
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            text={LL.common.controls.saveChanges()}
            type="submit"
            loading={isLoading}
            icon={<IconCheckmarkWhite />}
            form="ldap-settings-form"
          />
        </div>
      </header>
      <form
        id="ldap-settings-form"
        className="column-layout"
        onSubmit={handleSubmit(handleValidSubmit)}
      >
        <LdapSettingsLeft control={control} />
        <LdapSettingsRight control={control} />
      </form>
    </section>
  );
};
