import './style.scss';

import { useQuery } from '@tanstack/react-query';
import classNames from 'classnames';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo } from 'react';
import { useParams } from 'react-router';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../i18n/i18n-react';
import { Button } from '../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/types';
import { EditButton } from '../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../shared/components/layout/EditButton/EditButtonOption';
import { IconCheckmarkWhite, IconEdit } from '../../../shared/components/svg';
import { deviceBreakpoints } from '../../../shared/constants';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../shared/queries';
import { ProfileDetails } from './ProfileDetails/ProfileDetails';
import { UserAuthInfo } from './UserAuthInfo/UserAuthInfo';
import { UserDevices } from './UserDevices/UserDevices';
import { UserWallets } from './UserWallets/UserWallets';
import { UserYubiKeys } from './UserYubiKeys/UserYubiKeys';

export const UserProfile = () => {
  const toaster = useToaster();
  const { LL } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const { username: paramsUsername } = useParams();
  const currentUser = useAuthStore((state) => state.user);
  const editMode = useUserProfileStore((state) => state.editMode);
  const setUserProfileState = useUserProfileStore((state) => state.setState);
  const appSettings = useAppStore((state) => state.settings);
  const {
    user: { getUser },
  } = useApi();

  const username = useMemo(() => {
    if (paramsUsername) {
      return paramsUsername;
    } else {
      if (currentUser?.username) {
        return currentUser.username;
      }
    }
    throw Error('No username found.');
  }, [currentUser?.username, paramsUsername]);

  useQuery([QueryKeys.FETCH_USER_PROFILE, username], () => getUser(username), {
    onSuccess: (userProfile) => {
      setUserProfileState({ userProfile });
    },
    onError: (err) => {
      toaster.error(LL.userPage.messages.failedToFetchUserData());
      console.error(err);
    },
    refetchOnWindowFocus: true,
    enabled: !isUndefined(username),
  });

  useEffect(() => {
    if (currentUser?.username === username || !username) {
      setUserProfileState({ isMe: true });
    } else {
      setUserProfileState({ isMe: false });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <section id="user-profile-v2">
      <header className={classNames({ edit: editMode })}>
        {breakpoint === 'desktop' && (
          <h1>{editMode ? LL.userPage.title.edit() : LL.userPage.title.view()}</h1>
        )}
        <div className={classNames('controls', { edit: editMode })}>
          {editMode ? <EditModeControls /> : <ViewModeControls />}
        </div>
      </header>
      <div className="content">
        <div className="wide-cards">
          <ProfileDetails />
          <UserAuthInfo />
        </div>
        <div className="cards-1">
          <UserDevices />
        </div>
        <div className="cards-2">
          <UserWallets />
          {appSettings?.worker_enabled && <UserYubiKeys />}
        </div>
      </div>
    </section>
  );
};

const ViewModeControls = () => {
  const setUserProfileState = useUserProfileStore((state) => state.setState);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const { LL } = useI18nContext();
  return (
    <>
      <div className="right">
        <Button
          data-testid="edit-user"
          text={breakpoint === 'desktop' ? LL.userPage.controls.editButton() : undefined}
          icon={<IconEdit />}
          styleVariant={
            breakpoint === 'desktop'
              ? ButtonStyleVariant.STANDARD
              : ButtonStyleVariant.ICON
          }
          onClick={() => setUserProfileState({ editMode: true })}
        />
      </div>
    </>
  );
};

const EditModeControls = () => {
  const { LL } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const userProfile = useUserProfileStore((state) => state.userProfile);
  const isAdmin = useAuthStore((state) => state.isAdmin);
  const isMe = useUserProfileStore((state) => state.isMe);
  const setUserProfileState = useUserProfileStore((state) => state.setState);
  const setDeleteUserModalState = useModalStore((state) => state.setDeleteUserModal);
  const loading = useUserProfileStore((state) => state.loading);

  const submitSubject = useUserProfileStore((state) => state.submitSubject);

  const handleDeleteUser = () => {
    if (userProfile) {
      setDeleteUserModalState({ visible: true, user: userProfile.user });
    }
  };

  return (
    <>
      {isAdmin && !isMe && breakpoint === 'desktop' ? (
        <div className="left">
          <Button
            text={LL.userPage.controls.deleteAccount()}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.CONFIRM}
            onClick={handleDeleteUser}
          />
        </div>
      ) : null}
      <div className="right">
        {breakpoint !== 'desktop' && isAdmin && (
          <EditButton visible={isAdmin}>
            <EditButtonOption
              data-testid="user-edit-delete-acccount"
              text={LL.userPage.controls.deleteAccount()}
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              disabled={!isAdmin || isMe}
              onClick={handleDeleteUser}
            />
          </EditButton>
        )}
        <Button
          data-testid="user-edit-cancel"
          text={LL.form.cancel()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => {
            setUserProfileState({ editMode: false });
          }}
        />
        <Button
          data-testid="user-edit-save"
          text={LL.form.saveChanges()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          icon={<IconCheckmarkWhite />}
          onClick={() => submitSubject.next()}
          loading={loading}
        />
      </div>
    </>
  );
};
