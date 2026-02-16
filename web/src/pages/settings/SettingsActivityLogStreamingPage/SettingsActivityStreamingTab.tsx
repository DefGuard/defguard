import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import type { LicenseInfo } from '../../../shared/api/types';
import { businessBadgeProps } from '../../../shared/components/badges/BusinessBadge';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyState } from '../../../shared/defguard-ui/components/EmptyState/EmptyState';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import {
  getActivityLogStreamsQueryOptions,
  getLicenseInfoQueryOptions,
} from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { ActivityLogStreamTable } from './ActivityLogStreamTable';
import { AddLogStreamingModal } from './modals/AddDestinationModal/AddLogStreamingModal';
import { DeleteLogStreamingModal } from './modals/DeleteDestinationModal/DeleteLogStreamingModal';
import { EditLogStreamingModal } from './modals/EditDestinationModal/EditLogStreamingModal';

export const SettingsActivityLogStreamingPage = () => {
  const {
    data: licenseInfo,
    isFetched: licenseInfoFetched,
    isFetching: licenseInfoFetching,
  } = useQuery(getLicenseInfoQueryOptions);
  const { data: streams } = useQuery(getActivityLogStreamsQueryOptions);

  const isEmpty = !streams || streams.length === 0;

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: m.settings_activity_log_streaming_add_log_streaming_button(),
      iconLeft: 'file-add',
      testId: 'add-activity-stream',
      loading: licenseInfoFetching,
      onClick: () => {
        licenseActionCheck(
          canUseBusinessFeature(licenseInfo as LicenseInfo | null),
          () => {
            openModal(ModalName.AddLogStreaming);
          },
        );
      },
    }),
    [licenseInfo, licenseInfoFetching],
  );

  return (
    <SettingsLayout id="settings-activity-log-streaming-tab">
      <SettingsHeader
        badgeProps={
          !isPresent(licenseInfo) && licenseInfoFetched ? businessBadgeProps : undefined
        }
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
