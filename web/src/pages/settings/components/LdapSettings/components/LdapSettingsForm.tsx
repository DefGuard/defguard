import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';
import { type SubmitHandler, useForm } from 'react-hook-form';
import ReactMarkdown from 'react-markdown';
import { z } from 'zod';
import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import SvgIconX from '../../../../../shared/components/svg/IconX';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { MessageBox } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import {
  type SelectOption,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useSettingsPage } from '../../../hooks/useSettingsPage';
import { LdapConnectionTest } from './LdapConnectionTest';

const options: SelectOption<boolean>[] = [
  {
    value: false,
    label: 'Defguard',
    key: 0,
  },
  {
    value: true,
    label: 'LDAP',
    key: 1,
  },
];

export const LdapSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.ldapSettings;
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
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
          .trim()
          .url(LL.form.error.invalid())
          .min(1, LL.form.error.required()),
        ldap_bind_username: z.string().trim().min(1, LL.form.error.required()),
        ldap_bind_password: z.string().trim(),
        ldap_group_member_attr: z.string().trim().min(1, LL.form.error.required()),
        ldap_group_obj_class: z.string().trim().min(1, LL.form.error.required()),
        ldap_group_search_base: z.string().trim().min(1, LL.form.error.required()),
        ldap_groupname_attr: z.string().trim().min(1, LL.form.error.required()),
        ldap_member_attr: z.string().trim().min(1, LL.form.error.required()),
        ldap_user_obj_class: z.string().trim().min(1, LL.form.error.required()),
        ldap_user_auxiliary_obj_classes: z.string().trim(),
        ldap_user_search_base: z.string().trim().min(1, LL.form.error.required()),
        ldap_username_attr: z.string().trim().min(1, LL.form.error.required()),
        ldap_enabled: z.boolean(),
        ldap_sync_enabled: z.boolean(),
        ldap_is_authoritative: z.boolean(),
        ldap_use_starttls: z.boolean(),
        ldap_tls_verify_cert: z.boolean(),
        ldap_sync_interval: z.number().default(300),
        ldap_uses_ad: z.boolean(),
        ldap_user_rdn_attr: z.string().trim().optional(),
        ldap_sync_groups: z.string().trim(),
      }),
    [LL.form.error],
  );

  type FormFields = z.infer<typeof schema>;

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
      ldap_sync_groups: settings?.ldap_sync_groups.join(', ') ?? '',
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
      ldap_sync_groups: '',
    }),
    [],
  );

  const { handleSubmit, reset, control } = useForm({
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
      ldap_sync_groups: data.ldap_sync_groups
        .split(',')
        .map((group) => group.trim())
        .filter((group) => group.length > 0),
    };
    mutate(formattedData);
  };

  const handleDeleteSubmit = useCallback(() => {
    mutate({
      ...emptyValues,
      ldap_user_auxiliary_obj_classes: [],
      ldap_sync_groups: [],
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
        <div className="left">
          <div>
            <div className="subsection-header helper-row">
              <h3>{localLL.form.headings.connection_settings()}</h3>
              <Helper>{localLL.form.helpers.connection_settings()}</Helper>
            </div>
            <div className="checkbox-column">
              <FormCheckBox
                controller={{ control, name: 'ldap_enabled' }}
                label={localLL.form.labels.ldap_enable()}
                labelPlacement="right"
                disabled={!enterpriseEnabled}
              />
              <FormCheckBox
                controller={{ control, name: 'ldap_use_starttls' }}
                label={localLL.form.labels.ldap_use_starttls()}
                labelPlacement="right"
                disabled={!enterpriseEnabled}
              />
              <FormCheckBox
                controller={{ control, name: 'ldap_uses_ad' }}
                label={localLL.form.labels.ldap_uses_ad()}
                labelPlacement="right"
                disabled={!enterpriseEnabled}
              />
              <FormCheckBox
                controller={{ control, name: 'ldap_tls_verify_cert' }}
                label={localLL.form.labels.ldap_tls_verify_cert()}
                labelPlacement="right"
                disabled={!enterpriseEnabled}
              />
            </div>
            <FormInput
              controller={{ control, name: 'ldap_url' }}
              label={localLL.form.labels.ldap_url()}
              disabled={!enterpriseEnabled}
            />
            <FormInput
              controller={{ control, name: 'ldap_bind_username' }}
              label={localLL.form.labels.ldap_bind_username()}
              disabled={!enterpriseEnabled}
            />
            <FormInput
              controller={{ control, name: 'ldap_bind_password' }}
              label={localLL.form.labels.ldap_bind_password()}
              type="password"
              disabled={!enterpriseEnabled}
            />
            <FormInput
              controller={{ control, name: 'ldap_sync_groups' }}
              label={localLL.form.labels.ldap_sync_groups()}
              labelExtras={<Helper>{localLL.sync.helpers.groups()}</Helper>}
            />
          </div>
          <div>
            <div className="subsection-header helper-row">
              <h3>{localLL.form.headings.user_settings()}</h3>
              <Helper>{localLL.form.helpers.user_settings()}</Helper>
            </div>
            <FormInput
              controller={{ control, name: 'ldap_username_attr' }}
              label={localLL.form.labels.ldap_username_attr()}
              disabled={!enterpriseEnabled}
            />
            <FormInput
              controller={{ control, name: 'ldap_user_rdn_attr' }}
              label={localLL.form.labels.ldap_user_rdn_attr()}
              disabled={!enterpriseEnabled}
              labelExtras={<Helper>{localLL.form.helpers.ldap_user_rdn_attr()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'ldap_user_search_base' }}
              label={localLL.form.labels.ldap_user_search_base()}
              disabled={!enterpriseEnabled}
            />
            <FormInput
              controller={{ control, name: 'ldap_user_obj_class' }}
              label={localLL.form.labels.ldap_user_obj_class()}
              disabled={!enterpriseEnabled}
              labelExtras={<Helper>{localLL.form.helpers.ldap_user_obj_class()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'ldap_user_auxiliary_obj_classes' }}
              label={localLL.form.labels.ldap_user_auxiliary_obj_classes()}
              disabled={!enterpriseEnabled}
              labelExtras={
                <Helper>{localLL.form.helpers.ldap_user_auxiliary_obj_classes()}</Helper>
              }
            />
            <FormInput
              controller={{ control, name: 'ldap_member_attr' }}
              label={localLL.form.labels.ldap_member_attr()}
              disabled={!enterpriseEnabled}
            />
          </div>
        </div>
        <div className="right">
          <div>
            <div className="helper-row subsection-header">
              <h3>{localLL.form.headings.group_settings()}</h3>
              <Helper>{localLL.form.helpers.group_settings()}</Helper>
            </div>
            <FormInput
              controller={{ control, name: 'ldap_groupname_attr' }}
              label={localLL.form.labels.ldap_groupname_attr()}
              disabled={!enterpriseEnabled}
            />
            <FormInput
              controller={{ control, name: 'ldap_group_obj_class' }}
              label={localLL.form.labels.ldap_group_obj_class()}
              disabled={!enterpriseEnabled}
              labelExtras={<Helper>{localLL.form.helpers.ldap_group_obj_class()}</Helper>}
            />
            <FormInput
              controller={{ control, name: 'ldap_group_member_attr' }}
              label={localLL.form.labels.ldap_group_member_attr()}
              disabled={!enterpriseEnabled}
            />
            <FormInput
              controller={{ control, name: 'ldap_group_search_base' }}
              label={localLL.form.labels.ldap_group_search_base()}
              disabled={!enterpriseEnabled}
            />
          </div>
          <div>
            <div className="helper-row subsection-header">
              <h3>{localLL.sync.header()}</h3>
              <Helper>{localLL.sync.helpers.heading()}</Helper>
            </div>
            <MessageBox type={MessageBoxType.INFO}>
              <ReactMarkdown>{localLL.sync.info()}</ReactMarkdown>
            </MessageBox>
            <div className="checkbox-column">
              <div className="helper-row">
                <FormCheckBox
                  controller={{ control, name: 'ldap_sync_enabled' }}
                  label={localLL.form.labels.ldap_sync_enabled()}
                  labelPlacement="right"
                  disabled={!enterpriseEnabled}
                />
                <Helper>{localLL.sync.helpers.sync_enabled()}</Helper>
              </div>
            </div>
            <FormSelect
              controller={{ control, name: 'ldap_is_authoritative' }}
              sizeVariant={SelectSizeVariant.STANDARD}
              options={options}
              label={localLL.form.labels.ldap_authoritative_source()}
              labelExtras={<Helper>{localLL.sync.helpers.authority()}</Helper>}
              disabled={!enterpriseEnabled}
            />
            <FormInput
              controller={{ control, name: 'ldap_sync_interval' }}
              label={localLL.form.labels.ldap_sync_interval()}
              type="number"
              disabled={!enterpriseEnabled}
              labelExtras={<Helper>{localLL.sync.helpers.interval()}</Helper>}
            />
          </div>
        </div>
      </form>
    </section>
  );
};
