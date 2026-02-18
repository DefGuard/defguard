import { Fragment, type PropsWithChildren, useMemo } from 'react';
import './style.scss';
import dayjs from 'dayjs';
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
import { ProgressionBar } from '../../../../../../../shared/defguard-ui/components/ProgressionBar/ProgressionBar';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';

type Props = {
  licenseInfo: LicenseInfo;
};

export const SettingsLicenseInfoSection = ({ licenseInfo: license }: Props) => {
  const licenseTier = license.tier;
  return (
    <div className="license-general-info">
      <div className="top">
        <PropertyInfo title={`Current plan`}>
          {isPresent(licenseTier) && (
            <>
              <p>{licenseTier}</p>
              {license.expired && <Badge variant="critical" text="Expired" />}
              {!license.expired && <Badge variant="success" text="Active" />}
            </>
          )}
          {!isPresent(licenseTier) && (
            <div>
              <Badge text="Unknown" variant="critical" />
            </div>
          )}
        </PropertyInfo>
        <PropertyInfo title={`License type`}>
          <p>{license.subscription ? 'Subscription' : 'Offline'}</p>
        </PropertyInfo>
        <PropertyInfo title={`Support type`}>
          <p>{`Placeholder`}</p>
        </PropertyInfo>
        {!license.expired && isPresent(license.valid_until) && (
          <PropertyInfo title={`Valid until`}>
            <ValidUntil validUntil={license.valid_until} />
          </PropertyInfo>
        )}
      </div>
      <Divider spacing={ThemeSpacing.Xl} />
      {isPresent(license.limits) && (
        <Fragment>
          <p className="limits-label">{`Current plan limits`}</p>
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
    let res = untilDay.format('DD/MM/YYYY');
    if (diff > 0) {
      res += ` (${diff} ${diff !== 1 ? 'days' : 'day'} left)`;
    }
    return res;
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
        title="Added users"
        icon="users"
        value={limits.users.current}
        maxValue={limits.users.limit}
      />
      <LicenseLimitProgress
        title="VPN locations"
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
