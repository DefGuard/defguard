import './style.scss';

import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { BuiltByCard } from './BuiltByCard/BuiltByCard';
import { DefaultNetworkSelect } from './DefaultNetworkSelect/DefaultNetworkSelect';
import { LicenseCard } from './LicenseCard/LicenseCard';
import { LicenseModal } from './LicenseModal/LicenseModal';
import { ModulesCard } from './ModulesCard/ModulesCard';
import { SupportCard } from './SupportCard/SupportCard';
import { Web3Settings } from './Web3Settings/Web3Settings';

export const SettingsPage = () => {
  return (
    <PageContainer id="settings-page">
      <header>
        <h1>Defguard Global Settings</h1>
      </header>
      <div className="left">
        <ModulesCard />
        <DefaultNetworkSelect />
        <Web3Settings />
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
