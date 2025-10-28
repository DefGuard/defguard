import './style.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useParams } from '@tanstack/react-router';
import { trainCase } from 'change-case';
import { useEffect, useMemo, useRef, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Page } from '../../../shared/components/Page/Page';
import { Tabs } from '../../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../../shared/defguard-ui/components/Tabs/types';
import { useAuth } from '../../../shared/hooks/useAuth';
import { userProfileQueryOptions } from '../../../shared/query';
import { createUserProfileStore, UserProfileContext } from './hooks/useUserProfilePage';
import { ProfileDetailsTab } from './tabs/ProfileDetailsTab/ProfileDetailsTab';
import { ProfileDevicesTab } from './tabs/ProfileDevicesTab/ProfileDevicesTab';

const tabs = {
  Details: 'details',
  Devices: 'devices',
} as const;

type TabsValue = (typeof tabs)[keyof typeof tabs];

export const UserProfilePage = () => {
  const authUsername = useAuth((s) => s.user?.username as string);

  const [activeTab, setActiveTab] = useState<TabsValue>('details');
  const { username } = useParams({
    from: '/_authorized/user/$username',
  });

  const isSelf = useMemo(
    (): boolean => authUsername === username,
    [authUsername, username],
  );

  const { data: userProfile } = useSuspenseQuery(userProfileQueryOptions(username));

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
    }),
  ).current;

  const tabsConfiguration = useMemo(() => {
    const res: TabsItem[] = [
      {
        title: m.profile_tabs_details(),
        active: activeTab === 'details',
        onClick: () => setActiveTab('details'),
      },
      {
        title: m.profile_tabs_devices(),
        active: activeTab === tabs.Devices,
        onClick: () => setActiveTab(tabs.Devices),
      },
    ];
    return res;
  }, [activeTab]);

  const RenderActiveTab = useMemo(() => {
    switch (activeTab) {
      case 'details':
        return ProfileDetailsTab;
      case 'devices':
        return ProfileDevicesTab;
    }
  }, [activeTab]);

  useEffect(() => {
    store.setState({ profile: userProfile });
  }, [userProfile, store]);

  return (
    <UserProfileContext value={store}>
      <Page id="user-profile-page" title={pageTitle}>
        <Tabs items={tabsConfiguration} />
        <RenderActiveTab />
      </Page>
    </UserProfileContext>
  );
};
