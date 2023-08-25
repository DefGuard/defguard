import './styles.scss';

import { SmtpSettingsForm } from './components/SmtpSettingsForm/SmtpSettingsForm';
import { SmtpTest } from './components/SmtpTest/SmtpTest';
export const SmtpSettings = () => {
  return (
    <>
      <div className="left">
        <SmtpSettingsForm />
      </div>
      <div className="right">
        <SmtpTest />
      </div>
    </>
  );
};
