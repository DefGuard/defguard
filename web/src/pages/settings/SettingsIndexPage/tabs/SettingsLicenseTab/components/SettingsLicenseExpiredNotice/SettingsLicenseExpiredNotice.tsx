import { m } from '../../../../../../../paraglide/messages';
import { SettingsCard } from '../../../../../../../shared/components/SettingsCard/SettingsCard';
import { externalLink } from '../../../../../../../shared/constants';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import expiredImage from '../../assets/expired.png';

export const SettingsLicenseExpiredNotice = () => {
  return (
    <SettingsCard id="license-expired-notice">
      <div className="notice-track">
        <div className="image-track">
          <img src={expiredImage} alt="" />
        </div>
        <div className="content-track">
          <p className="title">{m.settings_license_expired_notice_title()}</p>
          <p className="description">{m.settings_license_expired_notice_description()}</p>
          <a
            href={externalLink.defguard.pricing}
            rel="noreferrer noopener"
            target="_blank"
          >
            <Button
              variant="outlined"
              text={m.settings_license_expired_notice_button()}
              iconRight="open-in-new-window"
            />
          </a>
        </div>
      </div>
    </SettingsCard>
  );
};
