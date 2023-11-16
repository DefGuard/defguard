import './style.scss';

import { LdapConnectionTest } from './components/LdapConnectionTest';
import { LdapSettingsForm } from './components/LdapSettingsForm';

export const LdapSettings = () => {
  return (
    <>
      <div className="left">
        <LdapSettingsForm />
      </div>
      <div className="right">
        <LdapConnectionTest />
      </div>
    </>
  );
};
