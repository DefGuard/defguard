import './style.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useParams } from '@tanstack/react-router';
import { useEffect, useMemo, useRef, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Page } from '../../../shared/components/Page/Page';
import { Tabs } from '../../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../../shared/defguard-ui/components/Tabs/types';
import { userProfileQueryOptions } from '../../../shared/query';
import { createUserProfileStore, UserProfileContext } from './hooks/useUserProfilePage';
import { ProfileDetailsTab } from './tabs/ProfileDetailsTab/ProfileDetailsTab';

const tabs = {
  Details: 'details',
} as const;

type TabsValue = (typeof tabs)[keyof typeof tabs];

export const UserProfilePage = () => {
  const [activeTab, setActiveTab] = useState<TabsValue>('details');
  const { username } = useParams({
    from: '/_authorized/user/$username',
  });

  const { data: userProfile } = useSuspenseQuery(userProfileQueryOptions(username));

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
    ];
    return res;
  }, [activeTab]);

  const RenderActiveTab = useMemo(() => {
    switch (activeTab) {
      case 'details':
        return ProfileDetailsTab;
    }
  }, [activeTab]);

  useEffect(() => {
    store.setState({ profile: userProfile });
  }, [userProfile, store]);

  return (
    <UserProfileContext value={store}>
      <Page id="user-profile-page">
        <Tabs items={tabsConfiguration} />
        <RenderActiveTab />
      </Page>
    </UserProfileContext>
  );
};
