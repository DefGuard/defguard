import { useVirtualizer } from '@tanstack/react-virtual';
import { useMemo, useRef } from 'react';

import { ListCellText } from '../../../shared/components/Layout/ListCellText/ListCellText';
import { ListHeader } from '../../../shared/components/Layout/ListHeader/ListHeader';
import { ListHeaderColumnConfig } from '../../../shared/components/Layout/ListHeader/types';
import { CheckBox } from '../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import { InteractionBox } from '../../../shared/defguard-ui/components/Layout/InteractionBox/InteractionBox';
import { ActivityMock } from '../useActivityMock';

type Props = {
  data: ActivityMock[];
};

export const ActivityList = ({ data }: Props) => {
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
    (): ListHeaderColumnConfig<ActivityMock>[] => [
      {
        label: 'Date',
        enabled: true,
        key: 'date',
        sortKey: 'date',
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
          activeKey="date"
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
                  <ListCellText text={activity.date} />
                </div>
                <div className="cell user">
                  <ListCellText text={activity.user} />
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
                  <ListCellText text={activity.details} withCopy />
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
