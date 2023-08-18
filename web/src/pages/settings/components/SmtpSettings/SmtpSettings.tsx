import './styles.scss';

import { SmtpEncryption } from './components/SmtpEncryption/SmtpEncryption';
import { SmtpSettingsForm } from './components/SmtpSettingsForm/SmtpSettingsForm';
import { SmtpTest } from './components/SmtpTest/SmtpTest';
export const SmtpSettings = () => {
  return (
    <>
      <div className="left">
        <SmtpSettingsForm />
        <SmtpEncryption />
      </div>
      <div className="right">
        <SmtpTest />
      </div>
    </>
  );
};
