import './style.scss';

import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { BrandingCard } from './BrandingCard/BrandingCard';
import { BuiltByCard } from './BuiltByCard/BuiltByCard';
import { DefaultNetworkSelect } from './DefaultNetworkSelect/DefaultNetworkSelect';
import { LicenseCard } from './LicenseCard/LicenseCard';
import { LicenseModal } from './LicenseModal/LicenseModal';
import { ModulesCard } from './ModulesCard/ModulesCard';
import { SupportCard } from './SupportCard/SupportCard';
import { Web3Settings } from './Web3Settings/Web3Settings';

export const SettingsPage = () => {
  const settings = useAppStore((state) => state.settings);
  return (
    <PageContainer id="settings-page">
      <header>
        <h1>{settings?.instance_name} Global Settings</h1>
      </header>
      <div className="left">
        <ModulesCard />
        <DefaultNetworkSelect />
        <Web3Settings />
        <BrandingCard />
      </div>
      <div className="right">
        <LicenseCard />
        <SupportCard />
        <BuiltByCard />
      </div>
      <LicenseModal />
    </PageContainer>
  );
};
