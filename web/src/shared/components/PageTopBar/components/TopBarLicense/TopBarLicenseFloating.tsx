import { type HTMLProps, useMemo } from 'react';
import { m } from '../../../../../paraglide/messages';
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
    if (license === null) return m.license_no_license();
    switch (license.tier) {
      case 'Business':
        return m.license_plan_usage_business();
      case 'Enterprise':
        return m.license_plan_usage_enterprise();
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
                label={m.settings_license_users_limit_label()}
              />
              <TopBarLicenseProgress
                icon={IconKind.LocationTracking}
                value={license.limits.locations.current}
                maxValue={license.limits.locations.limit}
                label={m.settings_license_locations_limit_label()}
              />
            </div>
          )}
          {!license.limits_exceeded && warning && (
            <p className="warning">{m.license_approaching_limits()}</p>
          )}
          {license.limits_exceeded && (
            <p className="critical">{m.license_capacity_reached()}</p>
          )}
        </>
      )}
      {!isPresent(license) && (
        <p className="no-license">{m.license_open_source_message()}</p>
      )}
      <a href={externalLink.defguard.pricing} rel="noopener noreferrer" target="_blank">
        <Button
          variant="primary"
          iconRight="open-in-new-window"
          text={
            license === null
              ? m.settings_license_try_business_button()
              : m.license_see_other_plans()
          }
        />
      </a>
    </div>
  );
};
