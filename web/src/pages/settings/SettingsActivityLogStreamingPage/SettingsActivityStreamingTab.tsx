import { useMemo } from 'react';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { EmptyState } from '../../../shared/defguard-ui/components/EmptyState/EmptyState';
import { businessPlanBadgeProps } from '../shared/consts';
import './style.scss';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import { AddLogStreamingModal } from './modals/AddDestinationModal/AddLogStreamingModal';

export const SettingsActivityLogStreamingPage = () => {
  const isEmpty = false;

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add log streaming',
      iconLeft: 'file-add',
      testId: 'add-activity-stream',
      onClick: () => {
        openModal(ModalName.AddLogStreaming);
      },
    }),
    [],
  );

  return (
    <SettingsLayout id="settings-activity-log-streaming-tab">
      <SettingsHeader
        badgeProps={businessPlanBadgeProps}
        icon="activity"
        title="Activity log streaming"
        subtitle="Monitor and export real-time activity logs from your Defguard instance. Stream events to external systems for auditing, analytics, or security monitoring."
      />
      <EmptyState
        id=""
        icon="log"
        title="You don't have any activity log upstreams."
        subtitle={`Click the button below to add an activity log provider and start streaming events.`}
        primaryAction={addButtonProps}
      />
      <AddLogStreamingModal />
    </SettingsLayout>
  );
};
