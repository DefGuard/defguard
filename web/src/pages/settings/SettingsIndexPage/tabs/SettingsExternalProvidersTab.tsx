import { useSuspenseQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { Suspense } from 'react';
import Skeleton from 'react-loading-skeleton';
import { m } from '../../../../paraglide/messages';
import { businessBadgeProps } from '../../../../shared/components/badges/BusinessBadge';
import {
  ContextualHelpKey,
  ContextualHelpSidebar,
} from '../../../../shared/components/ContextualHelp';
import { SettingsLayout } from '../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { getLicenseInfoQueryOptions } from '../../../../shared/query';
import { canUseBusinessFeature } from '../../../../shared/utils/license';

export const SettingsExternalProvidersTab = () => {
  return (
    <SettingsLayout
      suggestion={<ContextualHelpSidebar pageKey={ContextualHelpKey.SettingsIdentity} />}
    >
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
          title={m.settings_openid_providers_title()}
          content={m.settings_openid_providers_subtitle()}
          badgeProps={canUseBusiness ? undefined : businessBadgeProps}
        />
      </Link>
      <SizedBox height={ThemeSpacing.Xl} />
      <Link to="/settings/ldap">
        <SectionSelect
          image="ldap"
          title={m.settings_ldap_title()}
          content={m.settings_ldap_subtitle()}
          badgeProps={canUseBusiness ? undefined : businessBadgeProps}
        />
      </Link>
    </>
  );
};
