import { type PropsWithChildren, useMemo } from 'react';
import './style.scss';
import dayjs from 'dayjs';
import type { LicenseInfo } from '../../../../../../../shared/api/types';
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
              <Badge variant="success" text="Active" />
            </>
          )}
          {!isPresent(licenseTier) && (
            <div>
              <Badge text="Unknown" variant="critical" />
            </div>
          )}
        </PropertyInfo>
        <PropertyInfo title={`License type`}>
          <p>{`Offline`}</p>
        </PropertyInfo>
        <PropertyInfo title={`Support type`}>
          <p>{`Community support`}</p>
        </PropertyInfo>
        {!license.expired && (
          <PropertyInfo title={`Valid until`}>
            <ValidUntil validUntil={license.valid_until} />
          </PropertyInfo>
        )}
      </div>
      <Divider spacing={ThemeSpacing.Xl} />
      <p className="limits-label">{`Current plan limits`}</p>
      <SizedBox height={ThemeSpacing.Xl2} />
      <LimitsSection license={license} />
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
    return `${untilDay.format('DD/MM/YYYY')} (${diff} ${diff !== 1 ? 'days' : 'day'} left)`;
  }, [validUntil]);

  return <p>{display}</p>;
};

type LimitSectionProps = {
  license: LicenseInfo;
};

const LimitsSection = (_props: LimitSectionProps) => {
  return (
    <div className="license-limits">
      <LicenseLimitProgress title="Added users" icon="users" value={4} maxValue={10} />
      <LicenseLimitProgress
        title="VPN locations"
        icon="location-tracking"
        value={1}
        maxValue={3}
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
