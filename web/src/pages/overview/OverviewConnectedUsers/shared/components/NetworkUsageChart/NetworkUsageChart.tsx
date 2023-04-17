import dayjs from 'dayjs';
import { useMemo } from 'react';
import { Bar, BarChart, XAxis, YAxis } from 'recharts';

import { ColorsRGB } from '../../../../../../shared/constants';
import { NetworkDeviceStats } from '../../../../../../shared/types';
import { parseStatsForCharts } from '../../../../helpers/stats';

interface NetworkUsageProps {
  data: NetworkDeviceStats['stats'];
  width?: number;
  height?: number;
  hideX?: boolean;
  barSize?: number;
  dataMax?: string | number;
}

export const NetworkUsageChart = ({
  data,
  height = 15,
  width = 105,
  hideX = true,
  barSize = 2,
  dataMax = 'dataMax + 1',
}: NetworkUsageProps) => {
  const getFormattedData = useMemo(() => parseStatsForCharts(data), [data]);

  return (
    <div className="network-usage">
      <BarChart
        height={height}
        width={width}
        data={getFormattedData}
        margin={{ bottom: 0, left: 0, right: 0, top: 0 }}
        barGap={2}
        barSize={barSize}
      >
        <XAxis
          dataKey="collected_at"
          scale="time"
          type="number"
          height={16}
          width={80}
          padding={{ left: 15, right: 15 }}
          axisLine={{ stroke: ColorsRGB.GrayBorder }}
          tickLine={{ stroke: ColorsRGB.GrayBorder }}
          hide={hideX}
          tick={{ fontSize: 10, color: ColorsRGB.GrayLight }}
          tickFormatter={formatXTick}
          domain={['dataMin', 'dataMax + 1']}
          interval={'preserveStartEnd'}
        />
        <YAxis hide={true} padding={{ bottom: 2 }} domain={['dataMin', dataMax]} />
        <Bar dataKey="download" fill={ColorsRGB.Primary} />
        <Bar dataKey="upload" fill={ColorsRGB.Error} />
      </BarChart>
    </div>
  );
};

const formatXTick = (tickData: number) => dayjs.utc(tickData).local().format('HH:mm');
