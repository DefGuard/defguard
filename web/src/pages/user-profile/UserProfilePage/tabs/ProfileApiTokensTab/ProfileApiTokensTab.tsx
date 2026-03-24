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
import { ApiTokensTabAvailability, type ApiTokensTabAvailabilityValue } from '../types';
import { ProfileApiTokensTable } from './components/ProfileApiTokensTable/ProfileApiTokensTable';
import { AddApiTokenModal } from './modals/AddApiTokenModal/AddApiTokenModal';
import { RenameApiTokenModal } from './modals/RenameApiTokenModal/RenameApiTokenModal';

type Props = {
  availability: ApiTokensTabAvailabilityValue;
  isLoading: boolean;
};

export const ProfileApiTokensTab = ({ availability, isLoading }: Props) => {
  if (availability === ApiTokensTabAvailability.Hidden) {
    return null;
  }

  if (availability === ApiTokensTabAvailability.Loading || isLoading) {
    return (
      <EmptyStateFlexible icon="api-token" title={m.profile_api_tokens_loading_title()} />
    );
  }

  if (availability === ApiTokensTabAvailability.Unavailable) {
    return (
      <EmptyStateFlexible
        icon="api-token"
        title={m.profile_api_tokens_unavailable_title()}
        subtitle={m.profile_api_tokens_unavailable_subtitle()}
      />
    );
  }

  return <AvailableProfileApiTokensTab />;
};

const AvailableProfileApiTokensTab = () => {
  const username = useUserProfile((s) => s.user.username);
  const apiTokens = useUserProfile((s) => s.apiTokens);

  return (
    <>
      {apiTokens.length === 0 && (
        <EmptyStateFlexible
          icon="api-token"
          title={m.profile_api_tokens_empty_title()}
          subtitle={m.profile_api_tokens_empty_subtitle()}
          primaryAction={{
            iconLeft: 'add-token',
            testId: 'add-token',
            text: m.profile_api_tokens_add(),
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
          <ProfileTabHeader title={m.profile_api_tokens_title()}>
            <Button
              text={m.profile_api_tokens_add()}
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
