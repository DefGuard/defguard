import { BrandingSettings } from './components/BrandingSettings/BrandingSettings';
import { ModulesSettings } from './components/ModulesSettings/ModulesSettings';
import { Web3Settings } from './components/Web3Settings/Web3Settings';

export const GlobalSettings = () => (
  <>
    <div className="left">
      <BrandingSettings />
      <ModulesSettings />
    </div>
    <div className="right">
      <Web3Settings />
    </div>
  </>
);
