import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import {
  ClientTrafficPolicy,
  type GroupClientTrafficPolicies,
} from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import {
  ContextualHelpKey,
  ContextualHelpSidebar,
} from '../../../shared/components/ContextualHelp';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Page } from '../../../shared/components/Page/Page';
import type { SelectionOption } from '../../../shared/components/SelectionSection/type';
import { SelectMultiple } from '../../../shared/components/SelectMultiple/SelectMultiple';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Icon, type IconKindValue } from '../../../shared/defguard-ui/components/Icon';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { ThemeSpacing, ThemeVariable } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import {
  getEnterpriseSettingsQueryOptions,
  getGroupsInfoQueryOptions,
  getLicenseInfoQueryOptions,
} from '../../../shared/query';
import './style.scss';
import { Suspense, useMemo } from 'react';
import Skeleton from 'react-loading-skeleton';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { businessBadgeProps } from '../../../shared/components/badges/BusinessBadge';
import { Controls } from '../../../shared/components/Controls/Controls';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';

const breadcrumbs = [
  <Link to="/settings" search={{ tab: 'general' }} key={0}>
    {m.settings_breadcrumb_general()}
  </Link>,
  <Link to="/settings/client" key={1}>
    {m.settings_breadcrumb_client_behavior()}
  </Link>,
];

export const SettingsClientPage = () => {
  const { data: license } = useQuery(getLicenseInfoQueryOptions);
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout
        suggestion={<ContextualHelpSidebar pageKey={ContextualHelpKey.SettingsClient} />}
      >
        <SettingsHeader
          icon="user"
          title={m.settings_client_title()}
          subtitle={m.settings_client_subtitle()}
          badgeProps={
            license !== undefined && !canUseBusinessFeature(license).result
              ? businessBadgeProps
              : undefined
          }
        />
        <Suspense fallback={<Skeleton height={480} />}>
          <Content />
        </Suspense>
      </SettingsLayout>
    </Page>
  );
};

const formSchema = z.object({
  admin_device_management: z.boolean(),
  only_client_activation: z.boolean(),
  disable_tunnels: z.boolean(),
  client_traffic_policy: z.enum(ClientTrafficPolicy),
  group_client_traffic_policies: z.object({
    none: z.array(z.number()),
    disable_all_traffic: z.array(z.number()),
    force_all_traffic: z.array(z.number()),
  }),
});

type FormFields = z.infer<typeof formSchema>;

type GroupPolicy = keyof GroupClientTrafficPolicies;

const emptyGroupClientTrafficPolicies = {
  none: [],
  disable_all_traffic: [],
  force_all_traffic: [],
};

type GroupPolicyRowProps = {
  canEdit: boolean;
  content: string;
  title: string;
  icon: IconKindValue;
  options: SelectionOption<number>[];
  selected: number[];
  onSelectionChange: (value: number[]) => void;
  onEditUnavailable: () => void;
};

const getSelectedGroupsCounterText = (count: number) => {
  if (count === 1) return m.location_access_selected_group_count_one({ count });
  return m.location_access_selected_group_count_other({ count });
};

const getAvailableGroupOptions = (
  options: SelectionOption<number>[],
  policy: GroupPolicy,
  policies: GroupClientTrafficPolicies,
) => {
  const assignedToOtherPolicy = new Set(
    Object.entries(policies)
      .filter(([key]) => key !== policy)
      .flatMap(([, groupIds]) => groupIds),
  );

  return options.filter((option) => !assignedToOtherPolicy.has(option.id));
};

const GroupPolicyRow = ({
  content,
  title,
  icon,
  canEdit,
  onSelectionChange,
  options,
  selected,
  onEditUnavailable,
}: GroupPolicyRowProps) => (
  <div className="group-policy-row">
    <Icon icon={icon} size={20} staticColor={ThemeVariable.FgMuted} />
    <div className="group-policy-row-content">
      <p className="group-policy-title">{title}</p>
      <p className="group-policy-content">{content}</p>
      {canEdit ? (
        <SelectMultiple
          counterText={getSelectedGroupsCounterText}
          editText={m.settings_client_traffic_policy_edit_groups()}
          modalTitle={m.settings_client_traffic_policy_edit_groups()}
          onSelectionChange={onSelectionChange}
          onToggleChange={() => {}}
          options={options}
          selected={new Set(selected)}
          toggleValue={false}
        />
      ) : (
        <button
          className="select-multiple-edit"
          type="button"
          onClick={onEditUnavailable}
        >
          {m.settings_client_traffic_policy_edit_groups()}
        </button>
      )}
    </div>
  </div>
);

const Content = () => {
  const { data: licenseInfo } = useSuspenseQuery(getLicenseInfoQueryOptions);
  const { data: settings } = useSuspenseQuery(getEnterpriseSettingsQueryOptions);
  const { data: groups } = useSuspenseQuery(getGroupsInfoQueryOptions);

  const noLicense = !isPresent(licenseInfo);
  const canUseTrafficPolicies = canUseBusinessFeature(licenseInfo).result;
  const groupClientTrafficPolicies = canUseTrafficPolicies
    ? (settings.group_client_traffic_policies ?? emptyGroupClientTrafficPolicies)
    : emptyGroupClientTrafficPolicies;

  const { mutateAsync: patchSettings } = useMutation({
    mutationFn: api.settings.patchEnterpriseSettings,
    meta: {
      invalidate: [['settings_enterprise'], ['settings']],
    },
    onSuccess: () => {
      Snackbar.default(m.settings_msg_saved());
    },
    onError: () => {
      Snackbar.error(m.settings_msg_save_failed());
    },
  });

  const defaultValues = useMemo((): FormFields => {
    return {
      admin_device_management: settings.admin_device_management,
      only_client_activation: settings.only_client_activation,
      disable_tunnels: settings.disable_tunnels,
      client_traffic_policy: canUseTrafficPolicies
        ? settings.client_traffic_policy
        : ClientTrafficPolicy.None,
      group_client_traffic_policies: groupClientTrafficPolicies,
    };
  }, [
    settings.admin_device_management,
    settings.client_traffic_policy,
    settings.only_client_activation,
    settings.disable_tunnels,
    canUseTrafficPolicies,
    groupClientTrafficPolicies,
  ]);

  const groupOptions = groups.map<SelectionOption<number>>((group) => ({
    id: group.id,
    label: group.name,
  }));

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const licenseCheck = canUseBusinessFeature(licenseInfo);
      if (!licenseCheck.result) {
        licenseActionCheck(licenseCheck, () => {});
        return;
      }
      await patchSettings(value);
      form.reset(value);
    },
  });

  return (
    <SettingsCard id="settings-client-behavior-card">
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <MarkedSection icon="enrollment">
            <h3>{m.settings_client_section_permissions_title()}</h3>
            <DescriptionBlock title={m.settings_client_permissions_description_title()}>
              <p>{m.settings_client_permissions_description()}</p>
            </DescriptionBlock>
            <form.AppField name="admin_device_management">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  variant="toggle"
                  title={m.settings_client_device_management_title()}
                  content={m.settings_client_device_management_content()}
                />
              )}
            </form.AppField>
            <form.AppField name="only_client_activation">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  variant="toggle"
                  title={m.settings_client_wireguard_configuration_title()}
                  content={m.settings_client_wireguard_configuration_content()}
                />
              )}
            </form.AppField>
            <form.AppField name="disable_tunnels">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  variant="toggle"
                  title={m.settings_client_disable_tunnels_title()}
                  content={m.settings_client_disable_tunnels_content()}
                />
              )}
            </form.AppField>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="protection">
            <h3>{m.settings_client_section_traffic_policy_title()}</h3>
            <DescriptionBlock
              title={m.settings_client_traffic_policy_description_title()}
            >
              <p>{m.settings_client_traffic_policy_description()}</p>
            </DescriptionBlock>
            <form.AppField name="client_traffic_policy">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={!canUseTrafficPolicies}
                  value={ClientTrafficPolicy.None}
                  variant="radio"
                  title={m.settings_client_traffic_policy_none_title()}
                  content={m.settings_client_traffic_policy_none_content()}
                />
              )}
            </form.AppField>
            <form.AppField name="client_traffic_policy">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={!canUseTrafficPolicies}
                  value={ClientTrafficPolicy.DisableAllTraffic}
                  variant="radio"
                  title={m.settings_client_traffic_policy_disable_all_title()}
                  content={m.settings_client_traffic_policy_disable_all_content()}
                />
              )}
            </form.AppField>
            <form.AppField name="client_traffic_policy">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={!canUseTrafficPolicies}
                  value={ClientTrafficPolicy.ForceAllTraffic}
                  variant="radio"
                  title={m.settings_client_traffic_policy_force_all_title()}
                  content={m.settings_client_traffic_policy_force_all_content()}
                />
              )}
            </form.AppField>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="groups">
            <h3>{m.settings_client_traffic_policy_group_title()}</h3>
            <p className="group-policy-description">
              {m.settings_client_traffic_policy_group_description()}
            </p>
            <form.Subscribe
              selector={(state) => state.values.group_client_traffic_policies}
            >
              {(policies) => (
                <>
                  <GroupPolicyRow
                    canEdit={canUseTrafficPolicies}
                    content={m.settings_client_traffic_policy_group_none_content()}
                    title={m.groups_traffic_policy_none()}
                    icon="online"
                    onSelectionChange={(none) =>
                      form.setFieldValue('group_client_traffic_policies', {
                        ...policies,
                        none,
                      })
                    }
                    options={getAvailableGroupOptions(groupOptions, 'none', policies)}
                    selected={policies.none}
                    onEditUnavailable={() =>
                      licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {})
                    }
                  />
                  <GroupPolicyRow
                    canEdit={canUseTrafficPolicies}
                    content={m.settings_client_traffic_policy_group_disable_all_content()}
                    title={m.settings_client_traffic_policy_group_disable_all_title()}
                    icon="globe-denied"
                    onSelectionChange={(disable_all_traffic) =>
                      form.setFieldValue('group_client_traffic_policies', {
                        ...policies,
                        disable_all_traffic,
                      })
                    }
                    options={getAvailableGroupOptions(
                      groupOptions,
                      'disable_all_traffic',
                      policies,
                    )}
                    selected={policies.disable_all_traffic}
                    onEditUnavailable={() =>
                      licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {})
                    }
                  />
                  <GroupPolicyRow
                    canEdit={canUseTrafficPolicies}
                    content={m.settings_client_traffic_policy_group_force_all_content()}
                    title={m.settings_client_traffic_policy_group_force_all_title()}
                    icon="gateway"
                    onSelectionChange={(force_all_traffic) =>
                      form.setFieldValue('group_client_traffic_policies', {
                        ...policies,
                        force_all_traffic,
                      })
                    }
                    options={getAvailableGroupOptions(
                      groupOptions,
                      'force_all_traffic',
                      policies,
                    )}
                    selected={policies.force_all_traffic}
                    onEditUnavailable={() =>
                      licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {})
                    }
                  />
                </>
              )}
            </form.Subscribe>
          </MarkedSection>
          <form.Subscribe
            selector={(s) => ({
              isDefault: s.isDefaultValue || s.isPristine,
              isSubmitting: s.isSubmitting,
            })}
          >
            {({ isDefault, isSubmitting }) => (
              <Controls>
                <div className="right">
                  <Button
                    type="submit"
                    variant="primary"
                    text={m.controls_save_changes()}
                    disabled={isDefault}
                    loading={isSubmitting}
                  />
                </div>
              </Controls>
            )}
          </form.Subscribe>
        </form.AppForm>
      </form>
    </SettingsCard>
  );
};
