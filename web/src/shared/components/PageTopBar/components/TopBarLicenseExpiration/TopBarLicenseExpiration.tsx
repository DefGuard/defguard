import './style.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { Suspense, useMemo } from 'react';
import { externalLink } from '../../../../constants';
import { Icon, IconKind } from '../../../../defguard-ui/components/Icon';
import { ThemeVariable } from '../../../../defguard-ui/types';
import { isPresent } from '../../../../defguard-ui/utils/isPresent';
import { getLicenseInfoQueryOptions } from '../../../../query';
import { getLicenseState } from '../../../../utils/license';
import { TopBarElementSkeleton } from '../../TopBarElementSkeleton';

export const TopBarLicenseExpiration = () => {
  return (
    <Suspense fallback={<TopBarElementSkeleton />}>
      <Content />
    </Suspense>
  );
};

type MessageVariant = 'warning' | 'expired' | 'critical' | 'safe';

const Content = () => {
  const { data: license } = useSuspenseQuery(getLicenseInfoQueryOptions);
  const licenseState = getLicenseState(license);

  const isGracePeriod = licenseState === 'gracePeriod';

  const expiresDisplay = useMemo(() => {
    if (license === null || license.valid_until === null) return '';
    return dayjs.utc(license.valid_until).local().fromNow();
  }, [license]);

  const daysToEnd = useMemo(() => {
    if (!isPresent(license)) return null;
    if (license.expired || license.valid_until === null) return 0;
    const current = dayjs();
    const expires = dayjs.utc(license.valid_until).local();
    return expires.diff(current, 'days');
  }, [license]);

  const variant = useMemo((): MessageVariant => {
    if (!isPresent(license) || license.valid_until === null || daysToEnd === null)
      return 'safe';
    if (isGracePeriod) {
      return 'critical';
    }
    if (licenseState === 'expiredLicense') return 'expired';
    if (license.subscription) {
      return 'safe';
    }
    if (daysToEnd > 14) return 'safe';
    if (daysToEnd <= 14 && daysToEnd > 7) return 'warning';
    if (daysToEnd <= 7) return 'critical';
    return 'expired';
  }, [daysToEnd, isGracePeriod, license, licenseState]);

  if (!isPresent(license) || daysToEnd === null || variant === 'safe') return null;
  return (
    <div id="top-bar-license-expiration-warning">
      {variant === 'warning' && (
        <div>
          <Icon icon="warning-filled" staticColor={ThemeVariable.FgAttention} size={16} />
          <p>
            {`Your license expires on `}
            {dayjs(license.valid_until).format('ll')}
          </p>
          <UpdateLink />
        </div>
      )}
      {variant === 'critical' && (
        <div>
          <Icon
            icon="attention-filled"
            staticColor={ThemeVariable.FgCritical}
            size={16}
          />
          <p>
            <span className="critical">{`Action required: `}</span>
            {isGracePeriod ? `Your license expired ` : `Your license expires `}
            <strong>{expiresDisplay}</strong>
            {`.`}
          </p>
          <UpdateLink />
        </div>
      )}
      {variant === 'expired' && (
        <div>
          <Icon
            icon={IconKind.AttentionOutlined}
            staticColor={ThemeVariable.FgCritical}
            size={16}
          />
          <p className="critical">{`Your license has expired.`}</p>
          <UpdateLink />
        </div>
      )}
    </div>
  );
};

const UpdateLink = () => {
  return (
    <a
      target="_blank"
      href={externalLink.defguard.pricing}
      rel="noopener noreferrer"
    >{`Update`}</a>
  );
};
