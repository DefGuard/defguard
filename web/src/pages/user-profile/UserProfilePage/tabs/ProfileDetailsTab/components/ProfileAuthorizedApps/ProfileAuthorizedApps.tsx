import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { cloneDeep } from 'lodash-es';
import { useCallback } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import type { OAuth2AuthorizedApps, User } from '../../../../../../../shared/api/types';
import { IconButton } from '../../../../../../../shared/defguard-ui/components/IconButton/IconButton';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
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
            <>
              <AuthorizedApp data={app} key={app.oauth2client_id} />
            </>
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

  const { mutate } = useMutation({
    mutationFn: deleteAuthorizedApp,
    meta: {
      invalidate: [['oauth'], ['user', username]],
    },
  });

  return (
    <div className="app">
      <AuthorizedAppIconPlaceholder />
      <p>{data.oauth2client_name}</p>
      <div className="controls">
        <IconButton
          icon="delete"
          onClick={() => {
            mutate();
          }}
        />
      </div>
    </div>
  );
};
