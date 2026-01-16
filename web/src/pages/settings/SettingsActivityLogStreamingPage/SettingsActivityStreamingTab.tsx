import { useQuery } from '@tanstack/react-query';
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
import { getActivityLogStreamsQueryOptions } from '../../../shared/query';
import { ActivityLogStreamTable } from './ActivityLogStreamTable';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';

export const SettingsActivityLogStreamingPage = () => {
  const { data: streams } = useQuery(getActivityLogStreamsQueryOptions);

  const isEmpty = !streams || streams.length === 0;

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
      {isEmpty ? (
        <EmptyState
          id=""
          icon="log"
          title="You don't have any activity log upstreams."
          subtitle={`Click the button below to add an activity log provider and start streaming events.`}
          primaryAction={addButtonProps}
        />
      ) : (
        <>
          <TableTop text="All log streams">
            <Button {...addButtonProps} />
          </TableTop>
          <ActivityLogStreamTable data={streams} />
        </>
      )}
      <AddLogStreamingModal />
    </SettingsLayout>
  );
};
