import { useVirtualizer } from '@tanstack/react-virtual';
import dayjs from 'dayjs';
import { useMemo, useRef } from 'react';
import { useInView } from 'react-intersection-observer';

import { useI18nContext } from '../../../i18n/i18n-react';
import { ListCellText } from '../../../shared/components/Layout/ListCellText/ListCellText';
import { ListHeader } from '../../../shared/components/Layout/ListHeader/ListHeader';
import { ListHeaderColumnConfig } from '../../../shared/components/Layout/ListHeader/types';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { ListSortDirection } from '../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { ActivityLogEvent, ActivityLogSortKey } from '../../../shared/types';

type Props = {
  data: ActivityLogEvent[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  sortKey: ActivityLogSortKey;
  sortDirection: ListSortDirection;
  onNextPage: () => void;
  onSortChange: (
    sortKey: keyof ActivityLogEvent,
    sortDirection: ListSortDirection,
  ) => void;
};

export const ActivityList = ({
  data,
  isFetchingNextPage,
  hasNextPage,
  sortDirection,
  sortKey,
  onSortChange,
  onNextPage,
}: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.activity.list;
  const headersLL = localLL.headers;
  const { ref: infiniteLoadMoreElement } = useInView({
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
    (): ListHeaderColumnConfig<ActivityLogEvent>[] => [
      {
        label: headersLL.date(),
        enabled: true,
        key: 'date',
        sortKey: 'timestamp',
      },
      {
        label: headersLL.user(),
        key: 'user',
      },
      {
        label: headersLL.ip(),
        key: 'ip',
      },
      {
        label: headersLL.event(),
        key: 'event',
      },
      {
        label: headersLL.module(),
        key: 'module',
      },
      {
        label: headersLL.device(),
        key: 'device',
      },
    ],
    [headersLL],
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
          activeKey={sortKey}
          headers={listHeaders}
          sortDirection={sortDirection}
          onChange={onSortChange}
          selectAll={false}
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
                <div className="cell date">
                  <ListCellText
                    text={dayjs
                      .utc(activity.timestamp)
                      .local()
                      .format('YYYY-MM-DD HH:mm')}
                  />
                </div>
                <div className="cell user">
                  <ListCellText text={activity.username} />
                </div>
                <div className="cell ip">
                  <ListCellText text={activity.ip} />
                </div>
                <div className="cell event">
                  <ListCellText text={LL.enums.activityLogEventType[activity.event]()} />
                </div>
                <div className="cell module">
                  <ListCellText text={LL.enums.activityLogModule[activity.module]()} />
                </div>
                <div className="cell device">
                  <ListCellText text={activity.device} />
                </div>
              </div>
            );
          })}
          {hasNextPage && (
            <div className="end-row" ref={infiniteLoadMoreElement}>
              {isFetchingNextPage && <LoaderSpinner size={24} />}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
