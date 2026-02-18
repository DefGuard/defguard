import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { ClientTrafficPolicy } from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import {
  getEnterpriseSettingsQueryOptions,
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
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import { canUseBusinessFeature } from '../../../shared/utils/license';

const breadcrumbs = [
  <Link to="/settings" search={{ tab: 'general' }} key={0}>
    General
  </Link>,
  <Link to="/settings/client" key={1}>
    Client behavior
  </Link>,
];

export const SettingsClientPage = () => {
  const { data: license, isFetched } = useQuery(getLicenseInfoQueryOptions);
  return (
    <Page title="Settings">
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="user"
          title="Client behavior"
          subtitle="Manage user permissions and configuration options for device control, WireGuard setup, and VPN routing."
          badgeProps={!isPresent(license) && isFetched ? businessBadgeProps : undefined}
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
  client_traffic_policy: z.enum(ClientTrafficPolicy),
});

type FormFields = z.infer<typeof formSchema>;

const Content = () => {
  const { data: licenseInfo } = useSuspenseQuery(getLicenseInfoQueryOptions);
  const { data: settings } = useSuspenseQuery(getEnterpriseSettingsQueryOptions);

  const noLicense = !isPresent(licenseInfo);

  const { mutateAsync: patchSettings } = useMutation({
    mutationFn: api.settings.patchEnterpriseSettings,
    meta: {
      invalidate: [['enterprise_settings'], ['settings']],
    },
  });

  const defaultValues = useMemo((): FormFields => {
    return {
      admin_device_management: settings.admin_device_management,
      only_client_activation: settings.only_client_activation,
      client_traffic_policy: settings.client_traffic_policy,
    };
  }, [
    settings.admin_device_management,
    settings.client_traffic_policy,
    settings.only_client_activation,
  ]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      if (!licenseInfo) return;
      // only expire error is possible here
      const { result } = canUseBusinessFeature(licenseInfo);
      if (result) {
        await patchSettings(value);
      } else {
        openModal(ModalName.LicenseExpired, {
          licenseTier: licenseInfo?.tier,
        });
      }
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
            <h3>Permissions</h3>
            <DescriptionBlock title="Client Configuration Permissions">
              <p>
                Define which VPN client settings users can modify and which are
                restricted.
              </p>
            </DescriptionBlock>
            <form.AppField name="admin_device_management">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  variant="toggle"
                  title="Device management for users"
                  content="When this option is on, only Admins can manage devices in user profiles."
                />
              )}
            </form.AppField>
            <form.AppField name="only_client_activation">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  variant="toggle"
                  title="WireGuard configuration for users"
                  content="When this option is on, users can't view or download manual WireGuard configurations. Only Defguard desktop client setup will be available."
                />
              )}
            </form.AppField>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="protection">
            <h3>Client traffic policy</h3>
            <DescriptionBlock title="Client traffic rules">
              <p>
                Specify the conditions that determine how traffic should behave in the
                application.
              </p>
            </DescriptionBlock>
            <form.AppField name="client_traffic_policy">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  value={ClientTrafficPolicy.None}
                  variant="radio"
                  title="None"
                  content="When this option is enabled, users will be able to select all routing options."
                />
              )}
            </form.AppField>
            <form.AppField name="client_traffic_policy">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  value={ClientTrafficPolicy.DisableAllTraffic}
                  variant="radio"
                  title="Disable all traffic"
                  content="When this option is enabled, users will not be able to route all traffic through the VPN."
                />
              )}
            </form.AppField>
            <form.AppField name="client_traffic_policy">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  value={ClientTrafficPolicy.ForceAllTraffic}
                  variant="radio"
                  title="Force all traffic"
                  content="When this option is enabled, the users will always route all traffic through the VPN."
                />
              )}
            </form.AppField>
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
