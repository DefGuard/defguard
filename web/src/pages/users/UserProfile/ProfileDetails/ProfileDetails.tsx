import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import classNames from 'classnames';
import { useMemo } from 'react';
import Skeleton from 'react-loading-skeleton';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Label } from '../../../../shared/defguard-ui/components/Layout/Label/Label';
import { NoData } from '../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { Tag } from '../../../../shared/defguard-ui/components/Layout/Tag/Tag';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';
import { titleCase } from '../../../../shared/utils/titleCase';
import { ProfileDetailsForm } from './ProfileDetailsForm/ProfileDetailsForm';

export const ProfileDetails = () => {
  const { LL } = useI18nContext();
  const [editMode, userProfile] = useUserProfileStore(
    (state) => [state.editMode, state.userProfile],
    shallow,
  );
  return (
    <section id="profile-details">
      <header>
        <h2>{LL.userPage.userDetails.header()}</h2>
      </header>
      {userProfile && (
        <Card className={classNames({ edit: editMode })}>
          {editMode ? <ProfileDetailsForm /> : <ViewMode />}
        </Card>
      )}
      {!userProfile && <Skeleton className="profile-details" />}
    </section>
  );
};

const ViewMode = () => {
  const appSettings = useAppStore((state) => state.settings);
  const { LL } = useI18nContext();
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
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        toaster.success(LL.userPage.userDetails.messages.deleteApp());
      },
      onError: () => {
        toaster.error(LL.messages.error());
      },
    },
  );
  const user = useUserProfileStore((store) => store.userProfile?.user);
  const isMe = useUserProfileStore((store) => store.isMe);

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
          <Label>{LL.userPage.userDetails.fields.username.label()}</Label>
          <p>{user.username}</p>
        </div>
      </div>
      <div className="row">
        <div className="info">
          <Label>{LL.userPage.userDetails.fields.firstName.label()}</Label>
          <p>{user.first_name}</p>
        </div>
      </div>
      <div className="row">
        <div className="info">
          <Label>{LL.userPage.userDetails.fields.lastName.label()}</Label>
          <p>{user.last_name}</p>
        </div>
      </div>
      <div className="row">
        <div className="info">
          <Label>{LL.userPage.userDetails.fields.phone.label()}</Label>
          <p>{user.phone}</p>
        </div>
      </div>
      <div className="row">
        <div className="info">
          <Label>{LL.userPage.userDetails.fields.email.label()}</Label>
          <p>{user.email}</p>
        </div>
      </div>
      {!isMe && (
        <div className="row">
          <div className="info">
            <Label>{LL.userPage.userDetails.fields.status.label()}</Label>
            <p>
              {user.is_active
                ? LL.userPage.userDetails.fields.status.active()
                : LL.userPage.userDetails.fields.status.disabled()}
            </p>
          </div>
        </div>
      )}
      <div className="row tags">
        <Label>{LL.userPage.userDetails.fields.groups.label()}</Label>
        <div className="tags">
          {sortedGroups.map((group) => (
            <Tag disposable={false} text={titleCase(group)} key={group} />
          ))}
          {!sortedGroups.length && (
            <NoData customMessage={LL.userPage.userDetails.fields.groups.noData()} />
          )}
        </div>
      </div>
      {appSettings?.openid_enabled && (
        <div className="row tags">
          <Label>{LL.userPage.userDetails.fields.apps.label()}</Label>
          <div className="tags" data-testid="authorized-apps">
            {user?.authorized_apps?.map((app) => (
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
            {!(
              user.authorized_apps &&
              user?.authorized_apps.length &&
              user?.authorized_apps?.length > 0
            ) && <NoData customMessage={LL.userPage.userDetails.fields.apps.noData()} />}
          </div>
        </div>
      )}
    </>
  );
};
