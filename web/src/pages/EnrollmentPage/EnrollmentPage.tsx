import { useMemo, useState } from 'react';
import './style.scss';
import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { GeneralTab } from './tabs/GeneralTab';
import { MessageTemplatesTab } from './tabs/MessageTemplatesTab';

const EnrollmentPageTab = {
  General: 'general',
  MessageTemplates: 'message-templates',
} as const;

type EnrollmentTabValue = (typeof EnrollmentPageTab)[keyof typeof EnrollmentPageTab];

export const EnrollmentPage = () => {
  const [activeTab, setActiveTab] = useState<EnrollmentTabValue>(
    EnrollmentPageTab.General,
  );

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        title: m.settings_enrollment_tab_general(),
        active: activeTab === EnrollmentPageTab.General,
        onClick: () => {
          setActiveTab(EnrollmentPageTab.General);
        },
      },
      {
        title: m.settings_enrollment_tab_message_templates(),
        active: activeTab === EnrollmentPageTab.MessageTemplates,
        onClick: () => {
          setActiveTab(EnrollmentPageTab.MessageTemplates);
        },
      },
    ],
    [activeTab],
  );

  return (
    <Page id="enrollment-page" title={m.settings_enrollment_page_title()}>
      <SizedBox height={ThemeSpacing.Md} />
      <Tabs items={tabs} />
      {activeTab === EnrollmentPageTab.General && <GeneralTab />}
      {activeTab === EnrollmentPageTab.MessageTemplates && <MessageTemplatesTab />}
    </Page>
  );
};
