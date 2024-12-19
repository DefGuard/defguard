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
  heightX?: number;
}

export const NetworkUsageChart = ({
  data,
  height = 15,
  width = 105,
  hideX = true,
  barSize = 2,
  heightX = 20,
}: NetworkUsageProps) => {
  const getFormattedData = useMemo(() => parseStatsForCharts(data), [data]);

  return (
    <div className="network-usage">
      <BarChart
        height={height}
        width={width}
        data={getFormattedData}
        margin={{ bottom: 0, left: 0, right: 0, top: 0 }}
        barGap={0}
        barCategoryGap={0}
        barSize={barSize}
      >
        <XAxis
          dataKey="collected_at"
          scale="time"
          type="number"
          height={heightX}
          width={width}
          axisLine={{ stroke: ColorsRGB.GrayBorder }}
          tickLine={{ stroke: ColorsRGB.GrayBorder }}
          hide={hideX}
          padding={{ left: 0, right: 0 }}
          tick={{ fontSize: 10, color: ColorsRGB.GrayLight }}
          tickFormatter={formatXTick}
          domain={['dataMin', 'dataMax']}
          interval={'preserveStartEnd'}
        />
        <YAxis
          hide={true}
          domain={['dataMin', 'dataMax']}
          padding={{ top: 0, bottom: 0 }}
        />
        <Bar dataKey="download" fill={ColorsRGB.Primary} />
        <Bar dataKey="upload" fill={ColorsRGB.Error} />
      </BarChart>
    </div>
  );
};

const formatXTick = (tickData: number) => dayjs.utc(tickData).local().format('HH:mm');
