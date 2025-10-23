import './style.scss';
import { m } from '../../../../../../../paraglide/messages';
import type { OAuth2AuthorizedApps, User } from '../../../../../../../shared/api/types';
import { IconButton } from '../../../../../../../shared/defguard-ui/components/IconButton/IconButton';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { ProfileCard } from '../../../../components/ProfileCard/ProfileCard';
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
  return (
    <div className="app">
      <div className="app-icon">
        <AuthorizedAppIconPlaceholder />
      </div>
      <p>{data.oauth2client_name}</p>
      <div className="controls">
        <IconButton icon="delete" />
      </div>
    </div>
  );
};
