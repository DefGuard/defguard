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
import { EditLogStreamingModal } from './modals/EditDestinationModal/EditLogStreamingModal';
import { DeleteLogStreamingModal } from './modals/DeleteDestinationModal/DeleteLogStreamingModal';
import { getActivityLogStreamsQueryOptions } from '../../../shared/query';
import { ActivityLogStreamTable } from './ActivityLogStreamTable';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { m } from '../../../paraglide/messages';

export const SettingsActivityLogStreamingPage = () => {
  const { data: streams } = useQuery(getActivityLogStreamsQueryOptions);

  const isEmpty = !streams || streams.length === 0;

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: m.settings_activity_log_streaming_add_log_streaming_button(),
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
        title={m.settings_activity_log_streaming_title()}
        subtitle={m.settings_activity_log_streaming_description()}
      />
      {isEmpty ? (
        <EmptyState
          id="empty-state-upstreams"
          icon="log"
          title={m.settings_activity_log_streaming_no_upstreams()}
          subtitle={m.settings_activity_log_streaming_no_upstreams_subtitle()}
          primaryAction={addButtonProps}
        />
      ) : (
        <>
          <TableTop text={m.settings_activity_log_streaming_table_title()}>
            <Button {...addButtonProps} />
          </TableTop>
          <ActivityLogStreamTable data={streams} />
        </>
      )}
      <AddLogStreamingModal />
      <EditLogStreamingModal />
      <DeleteLogStreamingModal />
    </SettingsLayout>
  );
};
