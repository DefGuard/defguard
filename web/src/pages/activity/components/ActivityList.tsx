import { useVirtualizer } from '@tanstack/react-virtual';
import { useMemo, useRef } from 'react';
import { useInView } from 'react-intersection-observer';

import { ListCellText } from '../../../shared/components/Layout/ListCellText/ListCellText';
import { ListHeader } from '../../../shared/components/Layout/ListHeader/ListHeader';
import { ListHeaderColumnConfig } from '../../../shared/components/Layout/ListHeader/types';
import { CheckBox } from '../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import { InteractionBox } from '../../../shared/defguard-ui/components/Layout/InteractionBox/InteractionBox';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { AuditEvent } from '../../../shared/types';

type Props = {
  data: AuditEvent[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onNextPage: () => void;
};

export const ActivityList = ({
  data,
  isFetchingNextPage,
  hasNextPage,
  onNextPage,
}: Props) => {
  const { ref: chuj } = useInView({
    threshold: 0,
    trackVisibility: false,
    onChange: (inView) => {
      if (inView) {
        onNextPage();
      }
    },
  });
  const parentRef = useRef<HTMLDivElement>(null);
  const count = data.length;
  const virtualizer = useVirtualizer({
    count,
    estimateSize: () => 40,
    getScrollElement: () => parentRef.current,
    enabled: true,
    paddingStart: 45,
    paddingEnd: 10,
  });
  const items = virtualizer.getVirtualItems();
  const listHeaders = useMemo(
    (): ListHeaderColumnConfig<AuditEvent>[] => [
      {
        label: 'Date',
        enabled: true,
        key: 'date',
        sortKey: 'timestamp',
      },
      {
        label: 'User',
        key: 'user',
      },
      {
        label: 'IP',
        key: 'ip',
      },
      {
        label: 'Event',
        key: 'event',
      },
      {
        label: 'Module',
        key: 'module',
      },
      {
        label: 'Device',
        key: 'device',
      },
      {
        label: 'Details',
        key: 'details',
      },
    ],
    [],
  );
  return (
    <div className="virtual-list" ref={parentRef}>
      <div
        style={{
          height: virtualizer.getTotalSize(),
          width: '100%',
          position: 'relative',
        }}
      >
        <ListHeader
          activeKey="timestamp"
          headers={listHeaders}
          selectAll={false}
          onSelectAll={(val) => {
            console.log('Select all', val);
          }}
        />
        <div
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            transform: `translateY(${items[0]?.start ?? 0}px)`,
          }}
        >
          {items.map((virtualRow) => {
            const activity = data[virtualRow.index];
            return (
              <div
                className="list-row"
                key={virtualRow.key}
                data-index={virtualRow.index}
                ref={virtualizer.measureElement}
              >
                <div className="cell select-cell">
                  <InteractionBox onClick={() => {}}>
                    <CheckBox value={false} />
                  </InteractionBox>
                </div>
                <div className="cell date">
                  <ListCellText text={activity.timestamp} />
                </div>
                <div className="cell user">
                  <ListCellText text={String(activity.user_id)} />
                </div>
                <div className="cell ip">
                  <ListCellText text={activity.ip} />
                </div>
                <div className="cell event">
                  <ListCellText text={activity.event} />
                </div>
                <div className="cell module">
                  <ListCellText text={activity.module} />
                </div>
                <div className="cell device">
                  <ListCellText text={activity.device} />
                </div>
                <div className="cell details">
                  {isPresent(activity.details) && (
                    <ListCellText text={activity.details} withCopy />
                  )}
                </div>
              </div>
            );
          })}
          {hasNextPage && (
            <div className="end-row" ref={chuj}>
              {isFetchingNextPage && <LoaderSpinner size={24} />}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
