import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import { SettingsCard } from '../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../shared/components/SettingsLayout/SettingsLayout';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { ThemeSpacing } from '../../shared/defguard-ui/types';

const EnrollmentPageTab = {
  General: 'general',
  MessageTemplates: 'message-templates',
} as const;

type EnrollmentTabValue =
  (typeof EnrollmentPageTab)[keyof typeof EnrollmentPageTab];

export const EnrollmentPage = () => {
  const [activeTab, setActiveTab] =
    useState<EnrollmentTabValue>(EnrollmentPageTab.General);

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
      <SizedBox height={ThemeSpacing.Xl2} />
      <SettingsLayout>
        {activeTab === EnrollmentPageTab.General && (
          <SettingsHeader
            icon="key"
            title={m.settings_enrollment_general_title()}
            subtitle={m.settings_enrollment_page_subtitle()}
          />
        )}
        {activeTab === EnrollmentPageTab.MessageTemplates && (
          <SettingsHeader
            icon="activity-notes"
            title={m.settings_enrollment_message_templates_title()}
            subtitle={m.settings_enrollment_message_templates_subtitle()}
          />
        )}
        <SizedBox height={ThemeSpacing.Lg} />
        <SettingsCard>
          {activeTab === EnrollmentPageTab.General && <EmptyTabContent tab="general" />}
          {activeTab === EnrollmentPageTab.MessageTemplates && (
            <EmptyTabContent tab="message-templates" />
          )}
        </SettingsCard>
      </SettingsLayout>
    </Page>
  );
};

const EmptyTabContent = ({ tab }: { tab: EnrollmentTabValue }) => {
  return <div data-testid={`enrollment-tab-${tab}`} />;
};
