import './style.scss';

import { useMemo } from 'react';

import { Card } from '../../../../shared/components/layout/Card/Card';
import { Label } from '../../../../shared/components/layout/Label/Label';
import NoData from '../../../../shared/components/layout/NoData/NoData';
import { Tag } from '../../../../shared/components/layout/Tag/Tag';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import { sortByDate } from '../../../../shared/utils/sortByDate';
import { titleCase } from '../../../../shared/utils/titleCase';
import { ProfileDetailsForm } from './ProfileDetailsForm/ProfileDetailsForm';

export const ProfileDetails = () => {
  const editMode = useUserProfileV2Store((state) => state.editMode);
  return (
    <section id="profile-details">
      <h2>Profile Details</h2>
      <Card>{editMode ? <ProfileDetailsForm /> : <ViewMode />}</Card>
    </section>
  );
};

const ViewMode = () => {
  const user = useUserProfileV2Store((store) => store.user);

  const sortedGroups = useMemo(() => {
    if (user?.groups) {
      return user.groups.sort();
    }
    return [];
  }, [user?.groups]);

  const sortedAuthorizedApps = useMemo(() => {
    if (user && user.authorized_apps) {
      return sortByDate(user.authorized_apps, (app) => app.date, true);
    }
    return [];
  }, [user]);

  if (!user) return null;
  return (
    <>
      <div className="row">
        <div className="info">
          <Label>Username</Label>
          <p>{user.username}</p>
        </div>
      </div>
      <div className="row">
        <div className="info">
          <Label>First name</Label>
          <p>{user.first_name}</p>
        </div>
        <div className="info">
          <Label>Last name</Label>
          <p>{user.last_name}</p>
        </div>
      </div>
      <div className="row">
        <div className="info">
          <Label>Phone number</Label>
          <p>{user.phone}</p>
        </div>
        <div className="info">
          <Label>E-mail</Label>
          <p>{user.email}</p>
        </div>
      </div>
      <div className="row tags">
        <Label>User groups</Label>
        <div className="tags">
          {sortedGroups.map((group) => (
            <Tag disposable={false} text={titleCase(group)} key={group} />
          ))}
          {!sortedGroups.length && <NoData customMessage="No groups found." />}
        </div>
      </div>
      <div className="row tags">
        <Label>Authorized apps</Label>
        <div className="tags">
          {sortedAuthorizedApps.map((app) => (
            <Tag disposable={false} text={app.client_id} key={app.id} />
          ))}
          {!sortedAuthorizedApps.length && (
            <NoData customMessage="No apps authorized." />
          )}
        </div>
      </div>
    </>
  );
};
