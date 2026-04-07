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
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
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
    {m.settings_breadcrumb_identity_providers()}
  </Link>,
  <Link key={1} to="/settings/ldap">
    {m.settings_ldap_title()}
  </Link>,
];

export const SettingsLdapPage = () => {
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);

  const canUseFeature = useMemo(() => {
    if (licenseInfo === undefined) return undefined;
    return canUseBusinessFeature(licenseInfo).result;
  }, [licenseInfo]);

  return (
    <Page id="settings-ldap-page" title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbsLinks} />
      <SettingsLayout>
        <SettingsHeader
          icon={IconKind.Servers}
          title={m.settings_ldap_title()}
          subtitle={m.settings_ldap_subtitle()}
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
  ldap_user_auxiliary_obj_classes: z.string().trim().nullable(),
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
  ldap_sync_groups: z.string().trim().nullable(),
});

type FormFields = z.infer<typeof formSchema>;

const PageForm = () => {
  const isAppLdapEnabled = useApp((s) => s.appInfo.ldap_info.enabled);
  const { data: licenseInfo } = useSuspenseQuery(getLicenseInfoQueryOptions);
  const { data: settings } = useSuspenseQuery(getSettingsQueryOptions);

  const canUseBusinessLicenseCheck = useMemo(() => {
    if (licenseInfo === undefined) return false;
    return canUseBusinessFeature(licenseInfo).result;
  }, [licenseInfo]);

  const defaultValues = useMemo((): FormFields => {
    return {
      ldap_group_search_base: settings?.ldap_group_search_base ?? '',
      ldap_group_member_attr: settings?.ldap_group_member_attr ?? '',
      ldap_group_obj_class: settings?.ldap_group_obj_class ?? '',
      ldap_username_attr: settings?.ldap_username_attr ?? '',
      ldap_user_search_base: settings?.ldap_user_search_base ?? '',
      ldap_user_obj_class: settings?.ldap_user_obj_class ?? '',
      ldap_user_auxiliary_obj_classes:
        settings?.ldap_user_auxiliary_obj_classes.join(', ') || null,
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
      ldap_sync_groups: settings?.ldap_sync_groups.join(', ') || null,
    };
  }, [settings]);

  const { mutateAsync } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: [['settings'], ['info']],
    },
    onSuccess: () => {
      Snackbar.default(m.settings_msg_saved());
    },
    onError: (e) => {
      Snackbar.error(m.settings_msg_save_failed());
      console.error(e);
    },
  });

  const { mutate: handleLdapTest, isPending: testInProgress } = useMutation({
    mutationFn: api.settings.getLdapConnectionStatus,
    onSuccess: () => {
      Snackbar.default(m.settings_ldap_test_success());
    },
    onError: (e) => {
      Snackbar.error(m.settings_ldap_test_failed());
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
      const licenseCheckRes = canUseBusinessFeature(licenseInfo);
      if (!licenseCheckRes.result) {
        openModal(ModalName.UpgradeBusiness);
        return;
      }

      await mutateAsync({
        ...value,
        ldap_user_auxiliary_obj_classes: value.ldap_user_auxiliary_obj_classes
          ? value.ldap_user_auxiliary_obj_classes
              .split(',')
              .map((item) => item.trim())
              .filter(Boolean)
          : [],
        ldap_sync_groups: value.ldap_sync_groups
          ? value.ldap_sync_groups
              .split(',')
              .map((item) => item.trim())
              .filter(Boolean)
          : [],
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
            title={m.settings_ldap_section_connection_title()}
            description={m.settings_ldap_section_connection_description()}
          />
          <div className="checkbox-group-column">
            <form.AppField name="ldap_use_starttls">
              {(field) => (
                <field.FormCheckbox text={m.settings_ldap_checkbox_use_starttls()} />
              )}
            </form.AppField>
            <form.AppField name="ldap_uses_ad">
              {(field) => (
                <field.FormCheckbox
                  text={m.settings_ldap_checkbox_server_is_active_directory()}
                />
              )}
            </form.AppField>
            <form.AppField name="ldap_tls_verify_cert">
              {(field) => (
                <field.FormCheckbox
                  text={m.settings_ldap_checkbox_verify_tls_certificate()}
                />
              )}
            </form.AppField>
          </div>
          <SizedBox height={ThemeSpacing.Xl2} />
          <EvenSplit>
            <form.AppField name="ldap_url">
              {(field) => (
                <field.FormInput
                  label={m.form_label_url()}
                  required
                  notNull
                  helper={m.settings_ldap_helper_url()}
                />
              )}
            </form.AppField>
            <form.AppField name="ldap_bind_username">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_bind_username()}
                  helper={m.settings_ldap_helper_bind_username()}
                  required
                  notNull
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="ldap_bind_password">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_bind_password()}
                  helper={m.settings_ldap_helper_bind_password()}
                  required
                  notNull
                  type="password"
                />
              )}
            </form.AppField>
            <form.AppField name="ldap_sync_groups">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_sync_groups()}
                  helper={m.settings_ldap_helper_sync_groups()}
                />
              )}
            </form.AppField>
          </EvenSplit>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="add-user">
          <MarkedSectionHeader
            title={m.settings_ldap_section_user_title()}
            description={m.settings_ldap_section_user_description()}
          />
          <EvenSplit>
            <form.AppField name="ldap_username_attr">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_username_attribute()}
                  helper={m.settings_ldap_helper_username_attribute()}
                  required
                  notNull
                />
              )}
            </form.AppField>
            <form.AppField name="ldap_user_rdn_attr">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_user_rdn_attribute()}
                  helper={m.settings_ldap_helper_user_rdn_attribute()}
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="ldap_user_search_base">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_user_search_base()}
                  helper={m.settings_ldap_helper_user_search_base()}
                  required
                  notNull
                />
              )}
            </form.AppField>
            <form.AppField name="ldap_user_obj_class">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_user_object_class()}
                  helper={m.settings_ldap_helper_user_object_class()}
                  required
                  notNull
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="ldap_member_attr">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_member_attribute()}
                  helper={m.settings_ldap_helper_member_attribute()}
                  required
                  notNull
                />
              )}
            </form.AppField>
            <form.AppField name="ldap_user_auxiliary_obj_classes">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_additional_user_object_classes()}
                  helper={m.settings_ldap_helper_additional_user_object_classes()}
                />
              )}
            </form.AppField>
          </EvenSplit>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="groups">
          <MarkedSectionHeader
            title={m.settings_ldap_section_group_title()}
            description={m.settings_ldap_section_group_description()}
          />
          <EvenSplit>
            <form.AppField name="ldap_groupname_attr">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_groupname_attribute()}
                  helper={m.settings_ldap_helper_groupname_attribute()}
                  notNull
                  required
                />
              )}
            </form.AppField>
            <form.AppField name="ldap_group_obj_class">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_group_object_class()}
                  helper={m.settings_ldap_helper_group_object_class()}
                  notNull
                  required
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="ldap_group_member_attr">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_group_member_attribute()}
                  helper={m.settings_ldap_helper_group_member_attribute()}
                  notNull
                  required
                />
              )}
            </form.AppField>
            <form.AppField name="ldap_group_search_base">
              {(field) => (
                <field.FormInput
                  label={m.settings_ldap_label_group_search_base()}
                  helper={m.settings_ldap_helper_group_search_base()}
                  notNull
                  required
                />
              )}
            </form.AppField>
          </EvenSplit>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon={IconKind.Sync}>
          <MarkedSectionHeader
            title={m.settings_ldap_section_sync_title()}
            description={m.settings_ldap_section_sync_description()}
          />
          <form.AppField name="ldap_sync_enabled">
            {(field) => (
              <field.FormInteractiveBlock
                variant={'radio'}
                value={false}
                title={m.settings_ldap_sync_one_way_title()}
                content={m.settings_ldap_sync_one_way_content()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="ldap_sync_enabled">
            {(field) => (
              <field.FormInteractiveBlock
                variant={'radio'}
                value={true}
                title={m.settings_ldap_sync_two_way_title()}
                content={m.settings_ldap_sync_two_way_content()}
              />
            )}
          </form.AppField>
          <form.Subscribe selector={(s) => s.values.ldap_sync_enabled}>
            {(syncEnabled) => (
              <>
                <Fold open={syncEnabled}>
                  <Divider spacing={ThemeSpacing.Xl} />
                  <DescriptionBlock title={m.settings_ldap_authority_block_title()}>
                    <p>{m.settings_ldap_authority_block_description()}</p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="ldap_is_authoritative">
                    {(field) => (
                      <field.FormInteractiveBlock
                        variant={'radio'}
                        value={false}
                        title={m.settings_ldap_authority_defguard_title()}
                        content={m.settings_ldap_authority_defguard_content()}
                      />
                    )}
                  </form.AppField>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="ldap_is_authoritative">
                    {(field) => (
                      <field.FormInteractiveBlock
                        variant={'radio'}
                        value={true}
                        title={m.settings_ldap_authority_ldap_title()}
                        content={m.settings_ldap_authority_ldap_content()}
                      />
                    )}
                  </form.AppField>
                  <SizedBox height={ThemeSpacing.Xl2} />
                  <form.AppField name="ldap_sync_interval">
                    {(field) => (
                      <field.FormInput
                        notNull
                        label={m.settings_ldap_label_sync_interval()}
                        helper={m.settings_ldap_helper_sync_interval()}
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
            {(field) => (
              <field.FormToggle label={m.settings_ldap_toggle_enable_integration()} />
            )}
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
                  <TooltipProvider
                    disabled={
                      !(!isAppLdapEnabled || !isDefaultValue) ||
                      !canUseBusinessLicenseCheck
                    }
                  >
                    <TooltipTrigger>
                      <div>
                        <Button
                          type="button"
                          variant="outlined"
                          text={m.settings_ldap_button_test_connection()}
                          iconLeft={IconKind.Refresh}
                          disabled={
                            isSubmitting ||
                            !isDefaultValue ||
                            !isAppLdapEnabled ||
                            !canUseBusinessLicenseCheck
                          }
                          loading={testInProgress}
                          onClick={() => {
                            handleLdapTest();
                          }}
                        />
                      </div>
                    </TooltipTrigger>
                    <TooltipContent>
                      <p>{m.settings_ldap_test_connection_tooltip()}</p>
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
