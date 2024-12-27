import { BrandingSettings } from './components/BrandingSettings/BrandingSettings';
import { LicenseSettings } from './components/LicenseSettings/LicenseSettings';
import { ModulesSettings } from './components/ModulesSettings/ModulesSettings';

export const GlobalSettings = () => (
  <>
    <div className="left">
      <BrandingSettings />
      <ModulesSettings />
    </div>
    <div className="right">
      <LicenseSettings />
    </div>
  </>
);
