import { m } from '../../../../../paraglide/messages';
import { LayoutGrid } from '../../../../../shared/components/LayoutGrid/LayoutGrid';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { EmptyStateFlexible } from '../../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSize } from '../../../../../shared/defguard-ui/types';
import { openModal } from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import { ProfileTabHeader } from '../../components/ProfileTabHeader/ProfileTabHeader';
import { useUserProfile } from '../../hooks/useUserProfilePage';
import { AddAuthKeyModal } from './modals/AddAuthKeyModal/AddAuthKeyModal';
import { RenameAuthKeyModal } from './modals/RenameAuthKeyModal/RenameAuthKeyModal';
import { ProfileAuthKeysTable } from './ProfileAuthKeysTable';

export const ProfileAuthKeysTab = () => {
  const username = useUserProfile((s) => s.user.username);
  const userAuthKeys = useUserProfile((s) => s.authKeys);

  return (
    <>
      {userAuthKeys.length === 0 && (
        <EmptyStateFlexible
          icon="authentication"
          title={m.profile_auth_keys_no_data_title()}
          subtitle={m.profile_auth_keys_no_data_subtitle()}
          primaryAction={{
            text: m.profile_auth_keys_no_data_cta(),
            onClick: () => {
              openModal(ModalName.AddAuthKey, {
                username,
              });
            },
          }}
        />
      )}
      {userAuthKeys.length > 0 && (
        <LayoutGrid id="profile-auth-keys-tab">
          <SizedBox height={ThemeSize.Xl3} />
          <ProfileTabHeader title={m.profile_auth_keys_header_title()}>
            <Button
              variant="primary"
              text={m.profile_auth_keys_no_data_cta()}
              iconLeft="key"
              onClick={() => {
                openModal(ModalName.AddAuthKey, {
                  username,
                });
              }}
            />
          </ProfileTabHeader>
          <ProfileAuthKeysTable />
        </LayoutGrid>
      )}
      <AddAuthKeyModal />
      <RenameAuthKeyModal />
    </>
  );
};
