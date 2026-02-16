import { type HTMLProps, useMemo } from 'react';
import type { LicenseInfo } from '../../../../api/types';
import { externalLink } from '../../../../constants';
import { Button } from '../../../../defguard-ui/components/Button/Button';
import { IconKind } from '../../../../defguard-ui/components/Icon';
import { isPresent } from '../../../../defguard-ui/utils/isPresent';
import { TopBarLicenseProgress } from './components/TopBarLicenseProgress';

type Props = {
  license: LicenseInfo | null;
} & HTMLProps<HTMLDivElement>;

export const TopBarLicenseFloating = ({ license, ...props }: Props) => {
  const warning = useMemo(() => {
    if (!license || !license.limits) return false;
    if (
      license.limits.users.current > 0 &&
      license.limits.users.limit / license.limits.users.current < 2.0
    ) {
      return true;
    }
    if (
      license.limits.locations.current > 0 &&
      license.limits.locations.limit / license.limits.locations.current < 2.0
    ) {
      return true;
    }
    return false;
  }, [license]);

  const title = useMemo(() => {
    if (license === null) return 'No License';
    switch (license.tier) {
      case 'Business':
        return 'Business Plan usage';
      case 'Enterprise':
        return 'Enterprise Plan usage';
    }
  }, [license]);

  return (
    <div id="top-bar-license-floating" {...props}>
      <p className="title">{title}</p>
      {isPresent(license) && (
        <>
          {isPresent(license.limits) && (
            <div className="limits">
              <TopBarLicenseProgress
                icon={IconKind.Users}
                value={license.limits.users.current}
                maxValue={license.limits.users.limit}
                label="Added users"
              />
              <TopBarLicenseProgress
                icon={IconKind.LocationTracking}
                value={license.limits.locations.current}
                maxValue={license.limits.locations.limit}
                label="VPN locations"
              />
            </div>
          )}
          {!license.limits_exceeded && warning && (
            <p className="warning">{`You're approaching the limits of your current plan. To increase your limits, please upgrade to a higher-tier plan.`}</p>
          )}
          {license.limits_exceeded && (
            <p className="critical">{`You've reached your plan's maximum capacity. Upgrade today to avoid interruptions and gain more flexibility.`}</p>
          )}
        </>
      )}
      {!isPresent(license) && (
        <p className="no-license">
          {`You're using the open-source version with limited features.\n\nTo unlock more flexibility, upgrade your license to “Starter” for free.`}
        </p>
      )}
      <a href={externalLink.defguard.pricing} rel="noopener noreferrer" target="_blank">
        <Button
          variant="primary"
          iconRight="open-in-new-window"
          text={license === null ? `Upgrade for free` : `See other plans`}
        />
      </a>
    </div>
  );
};
