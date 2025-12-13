import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import type { OpenIdProviderSettings } from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { InteractiveBlock } from '../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getOpenIdProvidersQueryOptions } from '../../../shared/query';

const breadcrumbs = [
  <Link
    to="/settings"
    search={{
      tab: 'openid',
    }}
    key={0}
  >
    External Identity providers
  </Link>,
  <Link to="/settings" key={1}>
    General settings
  </Link>,
];

export const SettingsExternalOpenidGeneralPage = () => {
  const { data: providerResponse } = useQuery(getOpenIdProvidersQueryOptions);

  return (
    <Page title="Settings">
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title="General settings"
          subtitle="Here you can manage external identity settings."
        />
        <Content settings={providerResponse?.settings} />
      </SettingsLayout>
    </Page>
  );
};

const Content = ({ settings }: { settings?: OpenIdProviderSettings }) => {
  const disabled = !isPresent(settings);
  const [selected, setSelected] = useState<boolean>(settings?.create_account ?? false);

  useEffect(() => {
    setSelected(settings?.create_account ?? false);
  }, [settings?.create_account]);

  return (
    <SettingsCard>
      {disabled && (
        <>
          <InfoBanner
            icon="info-outlined"
            variant="warning"
            text="Please configure provider first"
          />
          <SizedBox height={ThemeSpacing.Xl} />
        </>
      )}
      <InteractiveBlock
        value={selected}
        disabled={disabled}
        variant="toggle"
        title="Automatically create user account when logging in for the first time through external OpenID."
        content="If this option is enabled, Defguard automatically creates new accounts for users who log in for the first time using an external OpenID. Otherwise, the user account must first be created by an administrator."
        onClick={() => {
          setSelected((s) => !s);
        }}
      />
    </SettingsCard>
  );
};
