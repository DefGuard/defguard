import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import classNames from 'classnames';
import { useMemo } from 'react';

import { Card } from '../../../../shared/components/layout/Card/Card';
import { Label } from '../../../../shared/components/layout/Label/Label';
import NoData from '../../../../shared/components/layout/NoData/NoData';
import { Tag } from '../../../../shared/components/layout/Tag/Tag';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';
import { titleCase } from '../../../../shared/utils/titleCase';
import { ProfileDetailsForm } from './ProfileDetailsForm/ProfileDetailsForm';

export const ProfileDetails = () => {
  const editMode = useUserProfileStore((state) => state.editMode);
  return (
    <section id="profile-details">
      <header>
        <h2>Profile Details</h2>
      </header>
      <Card className={classNames({ edit: editMode })}>
        {editMode ? <ProfileDetailsForm /> : <ViewMode />}
      </Card>
    </section>
  );
};
const ViewMode = () => {
  const {
    openid: { removeUserClient },
  } = useApi();

  const toaster = useToaster();
  const queryClient = useQueryClient();
  const { mutate: deleteTokenMutation } = useMutation(
    [MutationKeys.REMOVE_USER_CLIENT],
    removeUserClient,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        toaster.success('App and all tokens deleted');
      },
      onError: () => {
        toaster.error('App deletion failed');
      },
    }
  );
  const user = useUserProfileStore((store) => store.user);

  const sortedGroups = useMemo(() => {
    if (user?.groups) {
      return user.groups.sort();
    }
    return [];
  }, [user?.groups]);

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
          {user?.authorized_apps.map((app) => (
            <Tag
              disposable={true}
              text={app.oauth2client_name}
              key={app.oauth2client_id}
              onDispose={() =>
                deleteTokenMutation({
                  username: user.username,
                  client_id: app.oauth2client_id,
                })
              }
            />
          ))}
          {!(user.authorized_apps.length > 0) && (
            <NoData customMessage="No authorized apps." />
          )}
        </div>
      </div>
    </>
  );
};
