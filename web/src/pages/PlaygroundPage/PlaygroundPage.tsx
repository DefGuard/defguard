import './style.scss';

import { useMemo, useState } from 'react';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { PlaygroundGeneralTab } from './tabs/PlaygroundGeneral';
import { PlaygroundIcons } from './tabs/PlaygroundIcons.tsx/PlaygroundIcons';
import { PlaygroundMfa } from './tabs/PlaygroundMfa/PlaygroundMfa';
import { PlaygroundNew } from './tabs/PlaygroundNew/PlaygroundNew';

export const PlaygroundPage = () => {
  const [activeTab, setActiveTab] = useState(1);
  const tabs = useMemo(
    (): TabsItem[] => [
      {
        title: 'Icons',
        active: activeTab === 2,
        onClick: () => {
          setActiveTab(2);
        },
      },
      {
        title: 'General',
        active: activeTab === 0,
        onClick: () => {
          setActiveTab(0);
        },
      },
      {
        title: '2.0',
        active: activeTab === 1,
        onClick: () => {
          setActiveTab(1);
        },
      },
      {
        title: 'MFA',
        active: activeTab === 3,
        onClick: () => {
          setActiveTab(3);
        },
      },
    ],
    [activeTab],
  );
  return (
    <div id="playground-page">
      <Tabs items={tabs} />
      {activeTab === 0 && <PlaygroundGeneralTab />}
      {activeTab === 1 && <PlaygroundNew />}
      {activeTab === 2 && <PlaygroundIcons />}
      {activeTab === 3 && <PlaygroundMfa />}
    </div>
  );
};
