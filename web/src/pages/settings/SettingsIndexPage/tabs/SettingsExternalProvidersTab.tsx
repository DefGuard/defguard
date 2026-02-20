import { useSuspenseQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { Suspense } from 'react';
import Skeleton from 'react-loading-skeleton';
import { businessBadgeProps } from '../../../../shared/components/badges/BusinessBadge';
import { SettingsLayout } from '../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { getLicenseInfoQueryOptions } from '../../../../shared/query';
import { canUseBusinessFeature } from '../../../../shared/utils/license';

export const SettingsExternalProvidersTab = () => {
  return (
    <SettingsLayout>
      <Suspense fallback={<LoaderSkeleton />}>
        <Content />
      </Suspense>
    </SettingsLayout>
  );
};

const LoaderSkeleton = () => {
  return (
    <>
      <Skeleton height={120} />
      <SizedBox height={ThemeSpacing.Xl} />
      <Skeleton height={120} />
    </>
  );
};

const Content = () => {
  const { data: licenseInfo } = useSuspenseQuery(getLicenseInfoQueryOptions);

  const canUseBusiness = canUseBusinessFeature(licenseInfo).result;

  return (
    <>
      <Link to="/settings/openid">
        <SectionSelect
          image="external-id"
          title="OpenID"
          content="Manage user permissions and configuration options for device control, WireGuard setup, and VPN routing."
          badgeProps={canUseBusiness ? undefined : businessBadgeProps}
        />
      </Link>
      <SizedBox height={ThemeSpacing.Xl} />
      <Link to="/settings/ldap">
        <SectionSelect
          image="ldap"
          title="LDAP and Active Directory"
          content="Manage how and when your gateway sends notifications. Configure alert types, delivery methods, and recipients to stay informed about important events. "
          badgeProps={canUseBusiness ? undefined : businessBadgeProps}
        />
      </Link>
    </>
  );
};
