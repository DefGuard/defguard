import './LogDetailsDrawer.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Suspense, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { activityLogEventDisplay } from '../../../shared/api/activity-log-types';
import type { ActivityLogEvent } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { DrawerModal } from '../../../shared/defguard-ui/components/DrawerModal/DrawerModal';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/LoaderSpinner/LoaderSpinner';
import { Toggle } from '../../../shared/defguard-ui/components/Toggle/Toggle';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getActivityLogEventQueryOptions } from '../../../shared/query';
import { displayDate } from '../../../shared/utils/displayDate';
import { formatIpForDisplay } from '../../../shared/utils/formatIpForDisplay';
import { buildLogDetailsChanges, missingValuePlaceholder } from '../logDetails';

const logDetailsDateFormat = 'DD/MM/YYYY HH:mm';

/**
 * The changed settings live in the event metadata, which only the details endpoint
 * returns, so this is the one part of the drawer that waits on a request.
 */
const LogDetailsChanges = ({ eventId }: { eventId: number }) => {
  const [showUnchanged, setShowUnchanged] = useState(false);
  const { data: event } = useSuspenseQuery(getActivityLogEventQueryOptions(eventId));

  const changes = useMemo(() => buildLogDetailsChanges(event.metadata), [event.metadata]);
  const visibleChanges = showUnchanged
    ? changes
    : changes.filter((change) => change.changed);

  return (
    <>
      <div className="changes-header">
        <p className="changes-title">{m.activity_log_details_section_title()}</p>
        {changes.length > 0 && (
          <Toggle
            active={showUnchanged}
            label={m.activity_log_details_show_unchanged()}
            testId="show-unchanged-items"
            onClick={() => {
              setShowUnchanged((current) => !current);
            }}
          />
        )}
      </div>
      {visibleChanges.length > 0 ? (
        <div className="changes-table-container">
          <table className="changes-table">
            <thead>
              <tr>
                <th>{m.activity_log_details_col_changed()}</th>
                <th>{m.activity_log_details_col_from()}</th>
                <th>{m.activity_log_details_col_to()}</th>
              </tr>
            </thead>
            <tbody>
              {visibleChanges.map((change) => (
                <tr key={change.field}>
                  <td>{change.label}</td>
                  <td>{change.from}</td>
                  <td>{change.to}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <p className="changes-empty">
          {changes.length > 0
            ? m.activity_log_details_no_changes()
            : m.activity_log_details_no_metadata()}
        </p>
      )}
    </>
  );
};

const LogDetailsInfo = ({ event }: { event: ActivityLogEvent }) => {
  const rows = [
    {
      label: m.activity_log_details_label_date(),
      value: displayDate(event.timestamp, logDetailsDateFormat),
    },
    {
      label: m.activity_log_details_label_changed_by(),
      value: event.username,
    },
    {
      label: m.activity_log_details_label_ip(),
      value: isPresent(event.ip) ? formatIpForDisplay(event.ip) : missingValuePlaceholder,
    },
    {
      label: m.activity_log_details_label_location(),
      value: event.location ?? missingValuePlaceholder,
    },
    {
      label: m.activity_log_details_label_module(),
      value: event.module,
    },
  ];

  return (
    <>
      {rows.map((row) => (
        <div className="info-row" key={row.label}>
          <span className="info-label">{row.label}</span>
          <span className="info-value">{row.value}</span>
        </div>
      ))}
    </>
  );
};

type Props = {
  selectedRow: ActivityLogEvent | null;
  onClose: () => void;
};

export const LogDetailsDrawer = ({ selectedRow, onClose }: Props) => {
  return (
    <DrawerModal
      isOpen={selectedRow !== null}
      onClose={onClose}
      title={m.activity_log_details_title()}
      contentClassName="log-details-drawer"
    >
      {selectedRow && (
        <>
          <div className="log-details-drawer-body">
            <div className="drawer-block log-details-event">
              <p className="event-name">{activityLogEventDisplay[selectedRow.event]}</p>
              {isPresent(selectedRow.description) && (
                <p className="event-description">{selectedRow.description}</p>
              )}
            </div>
            <Divider />
            <div className="drawer-block log-details-info">
              <LogDetailsInfo event={selectedRow} />
            </div>
            <Divider />
            <div className="drawer-block log-details-changes">
              <Suspense
                fallback={
                  <div className="changes-loader">
                    <LoaderSpinner size={24} />
                  </div>
                }
              >
                <LogDetailsChanges eventId={selectedRow.id} />
              </Suspense>
            </div>
          </div>
          <div className="log-details-drawer-footer">
            <Button variant="secondary" text={m.controls_close()} onClick={onClose} />
          </div>
        </>
      )}
    </DrawerModal>
  );
};
