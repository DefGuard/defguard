import { Fragment, type PropsWithChildren, useMemo } from 'react';
import './style.scss';
import dayjs from 'dayjs';
import { m } from '../../../../../../../paraglide/messages';
import type {
  LicenseInfo,
  LicenseLimitsInfo,
} from '../../../../../../../shared/api/types';
import { Badge } from '../../../../../../../shared/defguard-ui/components/Badge/Badge';
import { Divider } from '../../../../../../../shared/defguard-ui/components/Divider/Divider';
import {
  Icon,
  type IconKindValue,
} from '../../../../../../../shared/defguard-ui/components/Icon';
import { InfoBanner } from '../../../../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { ProgressionBar } from '../../../../../../../shared/defguard-ui/components/ProgressionBar/ProgressionBar';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import type { LicenseState } from '../../../../../../../shared/utils/license';

type Props = {
  licenseInfo: LicenseInfo;
  licenseState: LicenseState;
};

export const SettingsLicenseInfoSection = ({
  licenseInfo: license,
  licenseState,
}: Props) => {
  const licenseTier = license.tier;
  const isGracePeriod = licenseState === 'gracePeriod';
  const isExpired = licenseState === 'expiredLicense';
  const isValid = licenseState === 'validBusiness' || licenseState === 'validEnterprise';
  const daysUntilExpiration = isPresent(license.valid_until)
    ? dayjs
        .utc(license.valid_until)
        .local()
        .startOf('day')
        .diff(dayjs().startOf('day'), 'day')
    : null;
  const isOfflineExpiringSoon =
    isValid &&
    !license.subscription &&
    daysUntilExpiration !== null &&
    daysUntilExpiration > 0 &&
    daysUntilExpiration <= 30;

  return (
    <div className="license-general-info">
      <div className="top">
        <PropertyInfo title={m.settings_license_current_plan()}>
          {isPresent(licenseTier) && (
            <>
              <p>{licenseTier}</p>
              {isExpired && <Badge variant="critical" text={m.misc_expired()} />}
              {isGracePeriod && <Badge variant="warning" text={m.misc_expired()} />}
              {isValid && <Badge variant="success" text={m.state_active()} />}
            </>
          )}
          {!isPresent(licenseTier) && (
            <div>
              <Badge text={m.settings_license_unknown()} variant="critical" />
            </div>
          )}
        </PropertyInfo>
        <PropertyInfo title={m.settings_license_type_title()}>
          <p>
            {license.subscription
              ? m.settings_license_subscription_type()
              : m.settings_license_offline_type()}
          </p>
        </PropertyInfo>
        <PropertyInfo title={m.settings_license_support_type_title()}>
          <p>{m.settings_license_support_type_value()}</p>
        </PropertyInfo>
        {isPresent(license.valid_until) && (
          <PropertyInfo title={m.settings_license_valid_until_title()}>
            <ValidUntil validUntil={license.valid_until} />
          </PropertyInfo>
        )}
      </div>
      {isExpired && (
        <>
          <SizedBox height={ThemeSpacing.Xl} />
          <InfoBanner
            icon="warning-filled"
            text={m.settings_license_expired_banner({ tier: license.tier })}
            variant="warning"
          />
        </>
      )}
      {!isExpired && isOfflineExpiringSoon && (
        <>
          <SizedBox height={ThemeSpacing.Xl} />
          <InfoBanner
            icon="warning-filled"
            text={m.settings_license_expiring_soon_banner({ days: daysUntilExpiration })}
            variant="warning"
          />
        </>
      )}
      <Divider spacing={ThemeSpacing.Xl} />
      {!isExpired && isPresent(license.limits) && (
        <Fragment>
          <p className="limits-label">{m.settings_license_limits_title()}</p>
          <SizedBox height={ThemeSpacing.Xl2} />
          <LimitsSection limits={license.limits} />
        </Fragment>
      )}
    </div>
  );
};

type ValidUntilProps = {
  validUntil: string;
};

const ValidUntil = ({ validUntil }: ValidUntilProps) => {
  const display = useMemo((): string => {
    const untilDay = dayjs.utc(validUntil).local();
    const nowDay = dayjs();
    const diff = untilDay.diff(nowDay, 'days');
    const formattedDate = untilDay.format('ll');

    if (diff > 0 && diff <= 28) {
      return m.settings_license_valid_until_with_time_left({
        date: formattedDate,
        duration: untilDay.fromNow(true),
      });
    }

    return formattedDate;
  }, [validUntil]);

  return <p>{display}</p>;
};

type LimitSectionProps = {
  limits: LicenseLimitsInfo;
};

const LimitsSection = ({ limits }: LimitSectionProps) => {
  return (
    <div className="license-limits">
      <LicenseLimitProgress
        title={m.settings_license_users_limit_label()}
        icon="users"
        value={limits.users.current}
        maxValue={limits.users.limit}
      />
      <LicenseLimitProgress
        title={m.settings_license_locations_limit_label()}
        icon="location-tracking"
        value={limits.locations.current}
        maxValue={limits.locations.limit}
      />
    </div>
  );
};

type PropertyInfoProps = {
  title: string;
} & PropsWithChildren;

const PropertyInfo = ({ title, children }: PropertyInfoProps) => {
  return (
    <div className="license-property-info">
      <div className="top">
        <p className="property-name">{title}</p>
      </div>
      <div className="bottom">{children}</div>
    </div>
  );
};

type LicenseLimitProgressProps = {
  title: string;
  icon: IconKindValue;
  value: number;
  maxValue: number;
};

const LicenseLimitProgress = ({
  icon,
  maxValue,
  title,
  value,
}: LicenseLimitProgressProps) => {
  return (
    <div className="license-limit-progress">
      <div className="info">
        <Icon icon={icon} />
        <p className="limit-name">{title}</p>
        <div className="counter">
          <p>{`${value}/${maxValue}`}</p>
        </div>
      </div>
      <SizedBox height={ThemeSpacing.Md} />
      <ProgressionBar value={value} maxValue={maxValue} />
      <SizedBox height={ThemeSpacing.Xl2} />
    </div>
  );
};
