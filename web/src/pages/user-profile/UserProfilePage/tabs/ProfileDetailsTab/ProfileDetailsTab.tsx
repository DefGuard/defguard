import './style.scss';
import { m } from '../../../../../paraglide/messages';
import { LayoutGrid } from '../../../../../shared/components/LayoutGrid/LayoutGrid';
import { EmptyState } from '../../../../../shared/defguard-ui/components/EmptyState/EmptyState';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { ProfileCard } from '../../components/ProfileCard/ProfileCard';
import { useUserProfile } from '../../hooks/useUserProfilePage';
import { ProfileAuthCard } from './components/ProfileAuthCard/ProfileAuthCard';
import { ProfileAuthorizedApps } from './components/ProfileAuthorizedApps/ProfileAuthorizedApps';
import { ProfileGeneralCard } from './components/ProfileGeneralCard/ProfileGeneralCard';
import { ChangePasswordModal } from './modals/ChangePasswordModal/ChangePasswordModal';
import { RecoveryCodesModal } from './modals/RecoveryCodesModal/RecoveryCodesModal';
import { TotpSetupModal } from './modals/TotpSetupModal/TotpSetupModal';

export const ProfileDetailsTab = () => {
  const authorizedApps = useUserProfile((s) => s.profile.user.authorized_apps);

  return (
    <LayoutGrid id="profile-details">
      <div className="left">
        <ProfileGeneralCard />
        {authorizedApps?.length === 0 && <AuthorizedAppsNoData />}
        {isPresent(authorizedApps) && authorizedApps.length > 0 && (
          <ProfileAuthorizedApps authorizedApps={authorizedApps} />
        )}
      </div>
      <ProfileAuthCard />
    </LayoutGrid>
  );
};

const AuthorizedAppsNoData = () => {
  return (
    <>
      <ProfileCard id="apps-no-data">
        <SizedBox height={ThemeSpacing.Xl5} />
        <EmptyState
          icon="apps"
          title={m.profile_apps_no_data_title()}
          subtitle={m.profile_apps_no_data_subtitle()}
        />
        <SizedBox height={ThemeSpacing.Xl5} />
      </ProfileCard>
      <ChangePasswordModal />
      <TotpSetupModal />
      <RecoveryCodesModal />
    </>
  );
};
