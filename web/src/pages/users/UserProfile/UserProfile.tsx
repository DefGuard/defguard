import './style.scss';

import { useQuery } from '@tanstack/react-query';
import classNames from 'classnames';
import { useEffect, useLayoutEffect } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router';
import useBreakpoint from 'use-breakpoint';

import { useI18nContext } from '../../../i18n/i18n-react';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { EditButton } from '../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../shared/components/layout/EditButton/EditButtonOption';
import { IconCheckmarkWhite, IconEdit } from '../../../shared/components/svg';
import { deviceBreakpoints } from '../../../shared/constants';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import { useNavigationStore } from '../../../shared/hooks/store/useNavigationStore';
import { useUserProfileStore } from '../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import { ProfileDetails } from './ProfileDetails/ProfileDetails';
import { UserAuthInfo } from './UserAuthInfo/UserAuthInfo';
import { UserDevices } from './UserDevices/UserDevices';
import { UserWallets } from './UserWallets/UserWallets';
import { UserYubiKeys } from './UserYubiKeys/UserYubiKeys';

export const UserProfile = () => {
  const { LL } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const location = useLocation();
  const { username } = useParams();
  const currentUser = useAuthStore((state) => state.user);
  const editMode = useUserProfileStore((state) => state.editMode);
  const setUserProfileState = useUserProfileStore((state) => state.setState);
  const setNavigationUser = useNavigationStore(
    (state) => state.setNavigationUser
  );
  const {
    user: { getUser },
  } = useApi();

  useQuery(
    [QueryKeys.FETCH_USER, username],
    () => getUser(username || (currentUser?.username as string)),
    {
      onSuccess: (user) => {
        setUserProfileState({ user: user });
        setNavigationUser(user);
      },
      refetchOnWindowFocus: false,
    }
  );

  useLayoutEffect(() => {
    if (location.pathname.includes('/edit')) {
      setUserProfileState({ editMode: true });
    } else {
      setUserProfileState({ editMode: false });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
          <h1>
            {editMode ? LL.userPage.title.edit() : LL.userPage.title.view()}
          </h1>
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
          <UserWallets />
        </div>
        <div className="cards-2">
          <UserYubiKeys />
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
          text={
            breakpoint === 'desktop'
              ? LL.userPage.controls.editButton()
              : undefined
          }
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
  const location = useLocation();
  const navigate = useNavigate();
  const user = useUserProfileStore((state) => state.user);
  const isAdmin = useAuthStore((state) => state.isAdmin);
  const isMe = useUserProfileStore((state) => state.isMe);
  const setUserProfileState = useUserProfileStore((state) => state.setState);
  const setDeleteUserModalState = useModalStore(
    (state) => state.setDeleteUserModal
  );

  const submitSubject = useUserProfileStore((state) => state.submitSubject);

  const handleDeleteUser = () => {
    if (user) {
      setDeleteUserModalState({ visible: true, user: user });
    }
  };

  return (
    <>
      {isAdmin && !isMe && breakpoint === 'desktop' ? (
        <div className="left">
          <Button
            text="Delete user"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.WARNING}
            onClick={handleDeleteUser}
          />
        </div>
      ) : null}
      <div className="right">
        {breakpoint !== 'desktop' && isAdmin && (
          <EditButton visible={isAdmin}>
            <EditButtonOption
              text={LL.userPage.controls.deleteAccount()}
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              disabled={!isAdmin || isMe}
              onClick={handleDeleteUser}
            />
          </EditButton>
        )}
        <Button
          text={LL.form.cancel()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => {
            if (location.pathname.includes('/edit')) {
              navigate('../');
            } else {
              setUserProfileState({ editMode: false });
            }
          }}
        />
        <Button
          text={LL.form.saveChanges()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.CONFIRM_SUCCESS}
          icon={<IconCheckmarkWhite />}
          onClick={async () => {
            setTimeout(() => submitSubject.next(), 500);
          }}
        />
      </div>
    </>
  );
};
