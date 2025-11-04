import './style.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate, useParams, useSearch } from '@tanstack/react-router';
import { trainCase } from 'change-case';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { m } from '../../../paraglide/messages';
import { Page } from '../../../shared/components/Page/Page';
import { Tabs } from '../../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../../shared/defguard-ui/components/Tabs/types';
import { useAuth } from '../../../shared/hooks/useAuth';
import {
  getUserApiTokensQueryOptions,
  getUserAuthKeysQueryOptions,
  userProfileQueryOptions,
} from '../../../shared/query';
import { createUserProfileStore, UserProfileContext } from './hooks/useUserProfilePage';
import { ProfileApiTokensTab } from './tabs/ProfileApiTokensTab/ProfileApiTokensTab';
import { ProfileAuthKeysTab } from './tabs/ProfileAuthKeysTab/ProfileAuthKeysTab';
import { ProfileDetailsTab } from './tabs/ProfileDetailsTab/ProfileDetailsTab';
import { ProfileDevicesTab } from './tabs/ProfileDevicesTab/ProfileDevicesTab';
import { UserProfileTab, type UserProfileTabValue } from './tabs/types';

const defaultTab = UserProfileTab.Details;

export const UserProfilePage = () => {
  const navigate = useNavigate();
  const authUsername = useAuth((s) => s.user?.username as string);
  const search = useSearch({ from: '/_authorized/user/$username' });
  const activeTab = search.tab ?? defaultTab;

  const { username } = useParams({
    from: '/_authorized/user/$username',
  });

  const isSelf = useMemo(
    (): boolean => authUsername === username,
    [authUsername, username],
  );

  const { data: userProfile } = useSuspenseQuery(userProfileQueryOptions(username));
  const { data: userAuthKeys } = useSuspenseQuery(getUserAuthKeysQueryOptions(username));
  const { data: userApiTokens } = useSuspenseQuery(
    getUserApiTokensQueryOptions(username),
  );

  const pageTitle = useMemo(() => {
    if (isSelf) {
      return m.profile_my_profile();
    }
    const name = trainCase(
      `${userProfile.user.first_name} ${userProfile.user.last_name}`,
    );
    return m.profile_title({
      name,
    });
  }, [isSelf, userProfile.user.first_name, userProfile.user.last_name]);

  const store = useRef(
    createUserProfileStore({
      profile: userProfile,
      authKeys: userAuthKeys,
      apiTokens: userApiTokens,
    }),
  ).current;

  const setActiveTab = useCallback(
    (tab: UserProfileTabValue) => {
      navigate({
        from: '/user/$username',
        to: '/user/$username',
        search: (perv) => ({ ...perv, tab }),
      });
    },
    [navigate],
  );

  const tabsConfiguration = useMemo(() => {
    const res: TabsItem[] = [
      {
        title: m.profile_tabs_details(),
        active: activeTab === UserProfileTab.Details,
        onClick: () => setActiveTab(UserProfileTab.Details),
      },
      {
        title: m.profile_tabs_devices(),
        active: activeTab === UserProfileTab.Devices,
        onClick: () => setActiveTab(UserProfileTab.Devices),
      },
      {
        title: m.profile_tabs_auth_keys(),
        active: activeTab === UserProfileTab.AuthKeys,
        onClick: () => setActiveTab(UserProfileTab.AuthKeys),
      },
      {
        title: m.profile_tabs_api(),
        active: activeTab === UserProfileTab.ApiTokens,
        onClick: () => setActiveTab(UserProfileTab.ApiTokens),
      },
    ];
    return res;
  }, [activeTab, setActiveTab]);

  const RenderActiveTab = useMemo(() => {
    switch (activeTab) {
      case UserProfileTab.Details:
        return ProfileDetailsTab;
      case UserProfileTab.Devices:
        return ProfileDevicesTab;
      case UserProfileTab.AuthKeys:
        return ProfileAuthKeysTab;
      case UserProfileTab.ApiTokens:
        return ProfileApiTokensTab;
    }
  }, [activeTab]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect
  useEffect(() => {
    if (store && userProfile) {
      store.setState({
        ...userProfile,
      });
    }
  }, [userProfile]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect
  useEffect(() => {
    if (store && userAuthKeys) {
      store.setState({
        authKeys: userAuthKeys,
      });
    }
  }, [userAuthKeys]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect
  useEffect(() => {
    if (store && userApiTokens) {
      store.setState({
        apiTokens: userApiTokens,
      });
    }
  }, [userApiTokens]);

  return (
    <UserProfileContext value={store}>
      <Page id="user-profile-page" title={pageTitle}>
        <Tabs items={tabsConfiguration} />
        <RenderActiveTab />
      </Page>
    </UserProfileContext>
  );
};
