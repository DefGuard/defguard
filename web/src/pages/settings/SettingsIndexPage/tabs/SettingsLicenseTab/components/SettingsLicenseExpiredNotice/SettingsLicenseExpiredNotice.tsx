import dayjs from 'dayjs';
import { m } from '../../../../../../../paraglide/messages';
import type { LicenseInfo } from '../../../../../../../shared/api/types';
import { SettingsCard } from '../../../../../../../shared/components/SettingsCard/SettingsCard';
import {
  externalLink,
  licenseGracePeriodDays,
} from '../../../../../../../shared/constants';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import expiredImage from '../../assets/expired.png';

type Props = {
  licenseInfo: LicenseInfo;
  state: 'gracePeriod' | 'expiredLicense';
};

export const SettingsLicenseExpiredNotice = ({ licenseInfo, state }: Props) => {
  const gracePeriodEndsAt = licenseInfo.valid_until
    ? dayjs.utc(licenseInfo.valid_until).local().add(licenseGracePeriodDays, 'day')
    : null;

  const gracePeriodDaysLeft = gracePeriodEndsAt
    ? Math.max(gracePeriodEndsAt.startOf('day').diff(dayjs().startOf('day'), 'day'), 0)
    : 0;

  const remainingDuration = m.settings_duration_days({ days: gracePeriodDaysLeft });

  const description =
    state === 'expiredLicense'
      ? m.settings_license_expired_notice_description_grace_period_ended()
      : m.settings_license_expired_notice_description_grace_period({
          duration: remainingDuration,
        });

  return (
    <SettingsCard id="license-expired-notice">
      <div className="notice-track">
        <div className="image-track">
          <img src={expiredImage} alt="" />
        </div>
        <div className="content-track">
          <p className="title">{m.settings_license_expired_notice_title()}</p>
          <p className="description">{description}</p>
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
