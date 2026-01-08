import { useMutation, useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { ClientTrafficPolicy, type SettingsEnterprise } from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { InteractiveBlock } from '../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getEnterpriseSettingsQueryOptions } from '../../../shared/query';
import './style.scss';
import { useEffect, useState } from 'react';
import api from '../../../shared/api/api';
import { higherPlanBadgeProps } from '../shared/consts';

const breadcrumbs = [
  <Link to="/settings" search={{ tab: 'general' }} key={0}>
    General
  </Link>,
  <Link to="/settings/client" key={1}>
    Client behavior
  </Link>,
];

export const SettingsClientPage = () => {
  const { data: settings } = useQuery(getEnterpriseSettingsQueryOptions);
  return (
    <Page title="Settings">
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="user"
          title="Client behavior"
          subtitle="Manage user permissions and configuration options for device control, WireGuard setup, and VPN routing."
          badgeProps={higherPlanBadgeProps}
        />
        {isPresent(settings) && <Content settings={settings} />}
      </SettingsLayout>
    </Page>
  );
};

const Content = ({ settings }: { settings: SettingsEnterprise }) => {
  const [adminDeviceManagement, setAdminDeviceManagment] = useState(
    settings.admin_device_management,
  );
  const [clientActivation, setClientActivation] = useState(
    settings.only_client_activation,
  );
  const [clientPolicy, setClientPolicy] = useState(settings.client_traffic_policy);

  const { mutate: patchSettings } = useMutation({
    mutationFn: api.settings.patchEnterpriseSettings,
    meta: {
      invalidate: ['enterprise_settings'],
    },
  });

  useEffect(() => {
    setAdminDeviceManagment(settings.admin_device_management);
    setClientActivation(settings.only_client_activation);
    setClientPolicy(settings.client_traffic_policy);
  }, [settings]);

  return (
    <SettingsCard id="settings-client-behavior-card">
      <MarkedSection icon="enrollment">
        <h3>Permissions</h3>
        <DescriptionBlock title="Client Configuration Permissions">
          <p>
            Define which VPN client settings users can modify and which are restricted.
          </p>
        </DescriptionBlock>
        <InteractiveBlock
          value={adminDeviceManagement}
          variant="toggle"
          title="Device management for users"
          content="When this option is on, only Admins can manage devices in user profiles."
          onClick={() => {
            const value = !adminDeviceManagement;
            setAdminDeviceManagment(value);
            patchSettings({
              admin_device_management: value,
            });
          }}
        />
        <InteractiveBlock
          value={clientActivation}
          variant="toggle"
          title="WireGuard configuration for users"
          content="When this option is on, users can't view or download manual WireGuard configurations. Only Defguard desktop client setup will be available."
          onClick={() => {
            const value = !clientActivation;
            setClientActivation(value);
            patchSettings({
              only_client_activation: value,
            });
          }}
        />
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
        <InteractiveBlock
          value={clientPolicy === ClientTrafficPolicy.None}
          variant="radio"
          title="None"
          content="When this option is enabled, users will be able to select all routing options."
          onClick={() => {
            setClientPolicy(ClientTrafficPolicy.None);
            patchSettings({
              client_traffic_policy: ClientTrafficPolicy.None,
            });
          }}
        />
        <InteractiveBlock
          value={clientPolicy === ClientTrafficPolicy.DisableAllTraffic}
          variant="radio"
          title="Disable all traffic"
          content="When this option is enabled, users will not be able to route all traffic through the VPN."
          onClick={() => {
            setClientPolicy(ClientTrafficPolicy.DisableAllTraffic);
            patchSettings({
              client_traffic_policy: ClientTrafficPolicy.DisableAllTraffic,
            });
          }}
        />
        <InteractiveBlock
          value={clientPolicy === ClientTrafficPolicy.ForceAllTraffic}
          variant="radio"
          title="Force all traffic"
          content="When this option is enabled, the users will always route all traffic through the VPN."
          onClick={() => {
            setClientPolicy(ClientTrafficPolicy.ForceAllTraffic);
            patchSettings({
              client_traffic_policy: ClientTrafficPolicy.ForceAllTraffic,
            });
          }}
        />
      </MarkedSection>
    </SettingsCard>
  );
};
