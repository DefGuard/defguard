import './style.scss';
import { cloneDeep } from 'lodash-es';
import { useCallback } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import type { OAuth2AuthorizedApps, User } from '../../../../../../../shared/api/types';
import { IconButton } from '../../../../../../../shared/defguard-ui/components/IconButton/IconButton';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { Snackbar } from '../../../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { ProfileCard } from '../../../../components/ProfileCard/ProfileCard';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';
import { AuthorizedAppIconPlaceholder } from './icons/AuthorizedAppIconPlaceholder';

export const ProfileAuthorizedApps = ({
  authorizedApps,
}: {
  authorizedApps: NonNullable<User['authorized_apps']>;
}) => {
  return (
    <ProfileCard id="authorized-apps-card">
      <h3>{m.profile_apps_card_title()}</h3>
      <p>{m.profile_apps_card_subtitle()}</p>
      {isPresent(authorizedApps) && (
        <div className="apps">
          {authorizedApps.map((app) => (
            <AuthorizedApp data={app} key={app.oauth2client_id} />
          ))}
        </div>
      )}
    </ProfileCard>
  );
};

type Props = {
  data: OAuth2AuthorizedApps;
};

const AuthorizedApp = ({ data }: Props) => {
  const username = useUserProfile((s) => s.user.username);

  const deleteAuthorizedApp = useCallback(async () => {
    const { data: userProfile } = await api.user.getUser(username);
    const clone = cloneDeep(userProfile.user);
    clone.authorized_apps = clone.authorized_apps ?? [];
    clone.authorized_apps = clone.authorized_apps.filter(
      (app) => app.oauth2client_id !== data.oauth2client_id,
    );
    await api.user.editUser({
      username,
      body: clone,
    });
  }, [data.oauth2client_id, username]);

  return (
    <div className="app">
      <AuthorizedAppIconPlaceholder />
      <p>{data.oauth2client_name}</p>
      <div className="controls">
        <IconButton
          icon="delete"
          onClick={() => {
            openModal(ModalName.ConfirmAction, {
              title: m.modal_delete_authorized_app_title(),
              contentMd: m.modal_delete_authorized_app(),
              actionPromise: deleteAuthorizedApp,
              invalidateKeys: [['oauth'], ['user', username]],
              submitProps: { text: m.controls_delete(), variant: 'critical' },
              onSuccess: () => Snackbar.default(m.modal_delete_authorized_app_success()),
              onError: () => Snackbar.error(m.modal_delete_authorized_app_error()),
            });
          }}
        />
      </div>
    </div>
  );
};
