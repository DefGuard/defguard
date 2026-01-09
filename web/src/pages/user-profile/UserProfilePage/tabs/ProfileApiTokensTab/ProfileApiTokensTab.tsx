import { m } from '../../../../../paraglide/messages';
import { LayoutGrid } from '../../../../../shared/components/LayoutGrid/LayoutGrid';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { EmptyStateFlexible } from '../../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { openModal } from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import { ProfileTabHeader } from '../../components/ProfileTabHeader/ProfileTabHeader';
import { useUserProfile } from '../../hooks/useUserProfilePage';
import { ProfileApiTokensTable } from './components/ProfileApiTokensTable/ProfileApiTokensTable';
import { AddApiTokenModal } from './modals/AddApiTokenModal/AddApiTokenModal';
import { RenameApiTokenModal } from './modals/RenameApiTokenModal/RenameApiTokenModal';

export const ProfileApiTokensTab = () => {
  const username = useUserProfile((s) => s.user.username);
  const apiTokens = useUserProfile((s) => s.apiTokens);
  return (
    <>
      {apiTokens.length === 0 && (
        <EmptyStateFlexible
          icon="api-token"
          title={m.profile_api_empty_title()}
          subtitle={m.profile_api_empty_subtitle()}
          primaryAction={{
            iconLeft: 'add-token',
            testId: 'add-token',
            text: m.profile_api_add(),
            onClick: () => {
              openModal(ModalName.AddApiToken, {
                username,
              });
            },
          }}
        />
      )}
      {apiTokens.length > 0 && (
        <LayoutGrid>
          <SizedBox height={ThemeSpacing.Xl3} />
          <ProfileTabHeader title={m.profile_api_title()}>
            <Button
              text={m.profile_api_add()}
              iconLeft="add-token"
              onClick={() => {
                openModal(ModalName.AddApiToken, {
                  username,
                });
              }}
            />
          </ProfileTabHeader>
          <ProfileApiTokensTable />
        </LayoutGrid>
      )}
      <AddApiTokenModal />
      <RenameApiTokenModal />
    </>
  );
};
