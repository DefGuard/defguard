import './LogDetailsDrawer.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { Suspense, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { activityLogEventDisplay } from '../../../shared/api/activity-log-types';
import type { ActivityLogEvent } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { DrawerModal } from '../../../shared/defguard-ui/components/DrawerModal/DrawerModal';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/LoaderSpinner/LoaderSpinner';
import { Toggle } from '../../../shared/defguard-ui/components/Toggle/Toggle';
import { TableBody } from '../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { renderTableCellValue } from '../../../shared/defguard-ui/components/table/utils/renderTableCellValue';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getActivityLogEventQueryOptions } from '../../../shared/query';
import { displayDate } from '../../../shared/utils/displayDate';
import { formatIpForDisplay } from '../../../shared/utils/formatIpForDisplay';
import {
  buildLogDetailsChanges,
  type LogDetailsChange,
  missingValuePlaceholder,
} from '../logDetails';

const logDetailsDateFormat = 'DD/MM/YYYY HH:mm';

const columnHelper = createColumnHelper<LogDetailsChange>();

/**
 * The changed settings live in the event metadata, which only the details endpoint
 * returns, so this is the one part of the drawer that waits on a request.
 */
const LogDetailsChanges = ({ eventId }: { eventId: number }) => {
  const [showUnchanged, setShowUnchanged] = useState(false);
  const { data: event } = useSuspenseQuery(getActivityLogEventQueryOptions(eventId));

  const changes = useMemo(() => buildLogDetailsChanges(event.metadata), [event.metadata]);
  const visibleChanges = useMemo(
    () => (showUnchanged ? changes : changes.filter((change) => change.changed)),
    [changes, showUnchanged],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('label', {
        header: m.activity_log_details_col_changed(),
        size: 180,
        minSize: 120,
        meta: { flex: true },
        cell: renderTableCellValue,
      }),
      columnHelper.accessor('from', {
        header: m.activity_log_details_col_from(),
        size: 240,
        minSize: 140,
        meta: { flex: true },
        cell: renderTableCellValue,
      }),
      columnHelper.accessor('to', {
        header: m.activity_log_details_col_to(),
        size: 240,
        minSize: 140,
        meta: { flex: true },
        cell: renderTableCellValue,
      }),
    ],
    [],
  );

  const table = useReactTable({
    columns,
    data: visibleChanges,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    getRowId: (change) => change.field,
  });

  return (
    <>
      <div className="header">
        <p className="title">{m.activity_log_details_section_title()}</p>
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
        <TableBody table={table} maxVisibleRows={8} />
      ) : (
        <p className="empty">
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
        <div className="row" key={row.label}>
          <span className="label">{row.label}</span>
          <span className="value">{row.value}</span>
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
          <div className="body">
            <div className="block event">
              <p className="name">{activityLogEventDisplay[selectedRow.event]}</p>
              {isPresent(selectedRow.description) && (
                <p className="description">{selectedRow.description}</p>
              )}
            </div>
            <Divider />
            <div className="block info">
              <LogDetailsInfo event={selectedRow} />
            </div>
            <Divider />
            <div className="block changes">
              <Suspense
                fallback={
                  <div className="loader">
                    <LoaderSpinner size={24} />
                  </div>
                }
              >
                <LogDetailsChanges eventId={selectedRow.id} />
              </Suspense>
            </div>
          </div>
          <div className="footer">
            <Button variant="secondary" text={m.controls_close()} onClick={onClose} />
          </div>
        </>
      )}
    </DrawerModal>
  );
};
