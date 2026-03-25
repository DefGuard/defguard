import dayjs from 'dayjs';
import { m } from '../../../../../../../paraglide/messages';
import type { LicenseInfo } from '../../../../../../../shared/api/types';
import { SettingsCard } from '../../../../../../../shared/components/SettingsCard/SettingsCard';
import {
  externalLink,
  licenseGracePeriodDays,
} from '../../../../../../../shared/constants';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import expiredImage from '../../assets/expired.png';

type Props = {
  licenseInfo: LicenseInfo;
  state: 'gracePeriod' | 'expiredLicense';
};

export const SettingsLicenseExpiredNotice = ({ licenseInfo, state }: Props) => {
  const gracePeriodDaysLeft = getGracePeriodDaysLeft(licenseInfo.valid_until);

  const remainingDuration = m.settings_duration_days({ days: gracePeriodDaysLeft });

  const description =
    state === 'expiredLicense'
      ? m.settings_license_expired_notice_description({ tier: licenseInfo.tier })
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
          <SizedBox height={ThemeSpacing.Xs} />
          <p className="description">{description}</p>
          <SizedBox height={ThemeSpacing.Md} />
          <a
            href={externalLink.defguard.pricing}
            rel="noreferrer noopener"
            target="_blank"
          >
            <Button
              variant="outlined"
              text={m.settings_license_expired_notice_button()}
            />
          </a>
        </div>
      </div>
    </SettingsCard>
  );
};

const getGracePeriodDaysLeft = (validUntil: string | null): number => {
  const gracePeriodEndsAt = validUntil
    ? dayjs.utc(validUntil).local().add(licenseGracePeriodDays, 'day')
    : null;

  return gracePeriodEndsAt
    ? Math.max(gracePeriodEndsAt.startOf('day').diff(dayjs().startOf('day'), 'day'), 0)
    : 0;
};
