import { Link } from '@tanstack/react-router';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import './style.scss';
import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { Suspense, useMemo } from 'react';
import Skeleton from 'react-loading-skeleton';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { businessBadgeProps } from '../../../shared/components/badges/BusinessBadge';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { EvenSplit } from '../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { IconKind } from '../../../shared/defguard-ui/components/Icon';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { TooltipContent } from '../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { useApp } from '../../../shared/hooks/useApp';
import {
  getLicenseInfoQueryOptions,
  getSettingsQueryOptions,
} from '../../../shared/query';
import { canUseBusinessFeature } from '../../../shared/utils/license';

const breadcrumbsLinks = [
  <Link
    key={0}
    to="/settings"
    search={{
      tab: 'identity',
    }}
  >
    Identity Providers
  </Link>,
  <Link key={1} to="/settings/ldap">
    LDAP and Active Directory
  </Link>,
];

export const SettingsLdapPage = () => {
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);

  const canUseFeature = useMemo(() => {
    if (licenseInfo === undefined) return undefined;
    return canUseBusinessFeature(licenseInfo).result;
  }, [licenseInfo]);

  return (
    <Page id="settings-ldap-page" title="Settings">
      <Breadcrumbs links={breadcrumbsLinks} />
      <SettingsLayout>
        <SettingsHeader
          icon={IconKind.Servers}
          title="LDAP and Active Directory"
          subtitle={`Manage LDAP/Active Directory connection details, user and group mapping, and synchronization rules that determine how users are imported and updated in Defguard.`}
          badgeProps={
            isPresent(canUseFeature) && !canUseFeature ? businessBadgeProps : undefined
          }
        />
        <Suspense fallback={<Skeleton height={500} />}>
          <SettingsCard>
            <PageForm />
          </SettingsCard>
        </Suspense>
      </SettingsLayout>
    </Page>
  );
};

const formSchema = z.object({
  ldap_bind_password: z.string().trim().nullable(),
  ldap_bind_username: z.string().trim().nullable(),
  ldap_url: z.string().trim(),
  ldap_group_member_attr: z.string().trim().min(1, m.form_error_required()),
  ldap_group_obj_class: z.string().trim().min(1, m.form_error_required()),
  ldap_group_search_base: z.string().trim().min(1, m.form_error_required()),
  ldap_groupname_attr: z.string().trim().min(1, m.form_error_required()),
  ldap_member_attr: z.string().trim().min(1, m.form_error_required()),
  ldap_user_obj_class: z.string().trim().min(1, m.form_error_required()),
  ldap_user_auxiliary_obj_classes: z.string().trim(),
  ldap_user_search_base: z.string().trim().min(1, m.form_error_required()),
  ldap_username_attr: z.string().trim().min(1, m.form_error_required()),
  ldap_enabled: z.boolean(),
  ldap_sync_enabled: z.boolean(),
  ldap_is_authoritative: z.boolean(),
  ldap_use_starttls: z.boolean(),
  ldap_tls_verify_cert: z.boolean(),
  ldap_sync_interval: z.number().min(
    10,
    m.form_error_min({
      value: 10,
    }),
  ),
  ldap_uses_ad: z.boolean(),
  ldap_user_rdn_attr: z.string().trim().nullable(),
  ldap_sync_groups: z.string().trim(),
});

type FormFields = z.infer<typeof formSchema>;

const PageForm = () => {
  const isAppLdapEnabled = useApp((s) => s.appInfo.ldap_info.enabled);
  const { data: settings } = useSuspenseQuery(getSettingsQueryOptions);

  const defaultValues = useMemo((): FormFields => {
    return {
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
    };
  }, [settings]);

  const { mutateAsync } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: [['settings'], ['info']],
    },
    onSuccess: () => {
      Snackbar.success(m.settings_msg_saved());
    },
    onError: (e) => {
      Snackbar.error('Failed to save settings.');
      console.error(e);
    },
  });

  const { mutate: handleLdapTest, isPending: testInProgress } = useMutation({
    mutationFn: api.settings.getLdapConnectionStatus,
    onSuccess: () => {
      Snackbar.success('LDAP Connected');
    },
    onError: (e) => {
      Snackbar.error('Connection failed');
      console.error(e);
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await mutateAsync({
        ...value,
        ldap_user_auxiliary_obj_classes: value.ldap_user_auxiliary_obj_classes
          .split(',')
          .map((item) => item.trim()),
        ldap_sync_groups: value.ldap_sync_groups.split(',').map((item) => item.trim()),
      });
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <MarkedSection icon={IconKind.NetworkSettings}>
          <MarkedSectionHeader
            title={`Connection settings`}
            description={`Configure LDAP connection settings here. These settings determine how Defguard connects to your LDAP server. Encrypted connections are also supported (StartTLS, LDAPS).`}
          />
          <div className="checkbox-group-column">
            <form.AppField name="ldap_use_starttls">
              {(field) => <field.FormCheckbox text={`Use StartTLS`} />}
            </form.AppField>
            <form.AppField name="ldap_uses_ad">
              {(field) => <field.FormCheckbox text={`LDAP server is Active Directory`} />}
            </form.AppField>
            <form.AppField name="ldap_tls_verify_cert">
              {(field) => <field.FormCheckbox text={`Verify TLS certificate`} />}
            </form.AppField>
          </div>
          <SizedBox height={ThemeSpacing.Xl2} />
          <EvenSplit>
            <form.AppField name="ldap_url">
              {(field) => <field.FormInput label="URL" required notNull />}
            </form.AppField>
            <form.AppField name="ldap_bind_username">
              {(field) => <field.FormInput label="Bind username" required notNull />}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="ldap_bind_password">
              {(field) => <field.FormInput label="Bind password" required notNull />}
            </form.AppField>
            <form.AppField name="ldap_sync_groups">
              {(field) => (
                <field.FormInput label="Limit synchronization to these groups" />
              )}
            </form.AppField>
          </EvenSplit>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="add-user">
          <MarkedSectionHeader
            title={`User settings`}
            description={`Configure LDAP user settings here. These settings determine how Defguard maps and synchronizes LDAP user information with local users.`}
          />
          <EvenSplit>
            <form.AppField name="ldap_username_attr">
              {(field) => <field.FormInput label="Username attribute" required notNull />}
            </form.AppField>
            <form.AppField name="ldap_user_rdn_attr">
              {(field) => <field.FormInput label="User RDN attribute" />}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="ldap_user_search_base">
              {(field) => <field.FormInput label="User search base" required notNull />}
            </form.AppField>
            <form.AppField name="ldap_user_obj_class">
              {(field) => <field.FormInput label="User object class" required notNull />}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="ldap_member_attr">
              {(field) => <field.FormInput label="Member attribute" required notNull />}
            </form.AppField>
            <form.AppField name="ldap_user_auxiliary_obj_classes">
              {(field) => (
                <field.FormInput label="Additional user object classes" notNull />
              )}
            </form.AppField>
          </EvenSplit>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="groups">
          <MarkedSectionHeader
            title={`Group settings`}
            description={`Configure LDAP group settings here. These settings determine how Defguard maps and synchronizes LDAP group information with local groups.`}
          />
          <EvenSplit>
            <form.AppField name="ldap_groupname_attr">
              {(field) => (
                <field.FormInput label="Groupname attribute" notNull required />
              )}
            </form.AppField>
            <form.AppField name="ldap_group_obj_class">
              {(field) => <field.FormInput label="Group object class" notNull required />}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="ldap_group_member_attr">
              {(field) => (
                <field.FormInput label="Group member attribute" notNull required />
              )}
            </form.AppField>
            <form.AppField name="ldap_group_search_base">
              {(field) => <field.FormInput label="Group search base" notNull required />}
            </form.AppField>
          </EvenSplit>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon={IconKind.Sync}>
          <MarkedSectionHeader
            title="LDAP synchronization"
            description="Control how Defguard synchronizes users with LDAP — disable sync, import users from LDAP, or keep both systems updated automatically."
          />
          <form.AppField name="ldap_sync_enabled">
            {(field) => (
              <field.FormInteractiveBlock
                variant={'radio'}
                value={false}
                title={`One-way synchronization`}
                content={`Users are imported from LDAP into Defguard. LDAP remains the source of truth and updates in the directory overwrite local data in Defguard.`}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="ldap_sync_enabled">
            {(field) => (
              <field.FormInteractiveBlock
                variant={'radio'}
                value={true}
                title={`Two-way synchronization`}
                content={`Defguard and LDAP stay in sync. Changes made in either system can be synchronized, and the selected authority decides which data wins if conflicts occur.`}
              />
            )}
          </form.AppField>
          <form.Subscribe selector={(s) => s.values.ldap_sync_enabled}>
            {(syncEnabled) => (
              <>
                <Fold open={syncEnabled}>
                  <Divider spacing={ThemeSpacing.Xl} />
                  <DescriptionBlock title="Source of authority">
                    <p>{`Select whether user data should be controlled by LDAP or by Defguard. The selected system becomes the primary source and can overwrite the other one when differences appear.`}</p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="ldap_is_authoritative">
                    {(field) => (
                      <field.FormInteractiveBlock
                        variant={'radio'}
                        value={false}
                        title={`Defguard is the source of truth`}
                        content={`When this option is enabled, users will be able to select all routing options.`}
                      />
                    )}
                  </form.AppField>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="ldap_is_authoritative">
                    {(field) => (
                      <field.FormInteractiveBlock
                        variant={'radio'}
                        value={true}
                        title={`LDAP is the source of truth`}
                        content={`LDAP directory data overrides Defguard records. User and group information in Defguard will be updated to match LDAP during synchronization.`}
                      />
                    )}
                  </form.AppField>
                  <SizedBox height={ThemeSpacing.Xl2} />
                  <form.AppField name="ldap_sync_interval">
                    {(field) => (
                      <field.FormInput
                        notNull
                        label="Synchronization interval (sec)"
                        type="number"
                        required={syncEnabled}
                      />
                    )}
                  </form.AppField>
                </Fold>
              </>
            )}
          </form.Subscribe>
        </MarkedSection>
        <Controls>
          <form.AppField name="ldap_enabled">
            {(field) => <field.FormToggle label={`Enable LDAP integration`} />}
          </form.AppField>
          <div className="right">
            <form.Subscribe
              selector={(s) => ({
                isSubmitting: s.isSubmitting,
                isDefaultValue: s.isPristine || s.isDefaultValue,
              })}
            >
              {({ isDefaultValue, isSubmitting }) => (
                <>
                  <TooltipProvider disabled={!(!isAppLdapEnabled || !isDefaultValue)}>
                    <TooltipTrigger>
                      <div>
                        <Button
                          type="button"
                          variant="outlined"
                          text={`Test connection`}
                          iconLeft={IconKind.Refresh}
                          disabled={isSubmitting || !isDefaultValue || !isAppLdapEnabled}
                          loading={testInProgress}
                          onClick={() => {
                            handleLdapTest();
                          }}
                        />
                      </div>
                    </TooltipTrigger>
                    <TooltipContent>
                      <p>{`To test connection please fill the form and save the changes first.`}</p>
                    </TooltipContent>
                  </TooltipProvider>
                  <Button
                    type="submit"
                    text={m.controls_save_changes()}
                    disabled={isDefaultValue}
                    loading={isSubmitting}
                  />
                </>
              )}
            </form.Subscribe>
          </div>
        </Controls>
      </form.AppForm>
    </form>
  );
};
