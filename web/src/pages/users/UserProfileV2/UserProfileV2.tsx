import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useEffect, useLayoutEffect } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { IconCheckmarkWhite, IconEdit } from '../../../shared/components/svg';
import { useAuthStore } from '../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import { useNavigationStore } from '../../../shared/hooks/store/useNavigationStore';
import { useUserProfileV2Store } from '../../../shared/hooks/store/useUserProfileV2Store';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import { ProfileDetails } from './ProfileDetails/ProfileDetails';
import { UserAuthInfo } from './UserAuthInfo/UserAuthInfo';
import { UserDevices } from './UserDevices/UserDevices';
import { UserWallets } from './UserWallets/UserWallets';
import { UserYubiKeys } from './UserYubiKeys/UserYubiKeys';

export const UserProfileV2 = () => {
  const location = useLocation();
  const { username } = useParams();
  const currentUser = useAuthStore((state) => state.user);
  const editMode = useUserProfileV2Store((state) => state.editMode);
  const setUserProfileState = useUserProfileV2Store((state) => state.setState);
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
      <header>
        <h1>{editMode ? 'Edit User Profile' : 'User Profile'}</h1>
        <div className="controls">
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
  const setUserProfileState = useUserProfileV2Store((state) => state.setState);
  return (
    <>
      <div className="right">
        <Button
          text="Edit profile"
          icon={<IconEdit />}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => setUserProfileState({ editMode: true })}
        />
      </div>
    </>
  );
};

const EditModeControls = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const user = useUserProfileV2Store((state) => state.user);
  const isAdmin = useAuthStore((state) => state.isAdmin);
  const isMe = useUserProfileV2Store((state) => state.isMe);
  const setUserProfileState = useUserProfileV2Store((state) => state.setState);
  const setDeleteUserModalState = useModalStore(
    (state) => state.setDeleteUserModal
  );

  const submitSubject = useUserProfileV2Store((state) => state.submitSubject);

  return (
    <>
      {isAdmin && !isMe ? (
        <div className="left">
          <Button
            text="Delete user"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.WARNING}
            onClick={() => {
              if (user) {
                setDeleteUserModalState({ visible: true, user: user });
              }
            }}
          />
        </div>
      ) : null}
      <div className="right">
        <Button
          text="Cancel"
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
          text="Save Changes"
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
