import './style.scss';

import { OpenIdGeneralSettings } from './components/OpenIdGeneralSettings';
import { OpenIdSettingsForm } from './components/OpenIdSettingsForm';

export const OpenIdSettings = () => {
  return (
    <>
      <div className="left">
        <OpenIdSettingsForm />
      </div>
      <div className="right">
        <OpenIdGeneralSettings />
      </div>
    </>
  );
};
