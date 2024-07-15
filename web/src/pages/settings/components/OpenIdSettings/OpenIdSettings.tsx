import './style.scss';

import { OpenIdSettingsForm } from './components/OpenIdSettingsForm';
import { OpenIdGeneralSettings } from './components/OpenIdGeneralSettings';

export const OpenIdSettings = () => {
  return (
    <>
      <div className="left">
        <OpenIdGeneralSettings />
      </div>
      <div className="right">
        <OpenIdSettingsForm />
      </div>
    </>
  );
};
