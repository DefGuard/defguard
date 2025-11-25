import { Bar, BarChart, ResponsiveContainer, XAxis, YAxis } from 'recharts';
import type { TransferChartData } from '../../utils/stats';
import './style.scss';
import dayjs from 'dayjs';
import { ThemeVariable } from '../../defguard-ui/types';

type Props = {
  data: TransferChartData[];
  height?: number;
  showX?: boolean;
  barGap?: number;
};

export const TransferChart = ({ data, showX, height = 50, barGap = 4 }: Props) => {
  return (
    <div className="transfer-chart">
      <ResponsiveContainer width="100%" height={height}>
        <BarChart
          data={data}
          barGap={barGap}
          barSize={2}
          margin={{ bottom: 0, left: 0, right: 0, top: 0 }}
        >
          {showX && (
            <XAxis
              dataKey="timestamp"
              tickFormatter={(timestamp) => dayjs.unix(timestamp).format('HH:mm')}
              tickLine={false}
              axisLine={false}
              padding={{
                left: 0,
                right: 0,
              }}
              tickMargin={8}
              height={23}
            />
          )}
          <YAxis tick={false} axisLine={false} tickLine={false} width={0} />
          <Bar dataKey="upload" width={2} radius={100} fill={ThemeVariable.FgCritical} />
          <Bar dataKey="download" width={2} radius={100} fill={ThemeVariable.FgAction} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
};
