import './style.scss';
import { useMemo } from 'react';
import { Bar, BarChart } from 'recharts';
import type { TransferStats } from '../../../../shared/api/types';
import { TransferText } from '../../../../shared/components/TransferText/TransferText';
import { TableCell } from '../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { ThemeVariable } from '../../../../shared/defguard-ui/types';
import { mapTransferToChart } from '../../../../shared/utils/stats';

export const DeviceTrafficChartCell = ({
  stats,
  download,
  upload,
}: {
  stats: TransferStats[];
  upload: number;
  download: number;
}) => {
  const traffic = useMemo(() => mapTransferToChart(stats), [stats]);

  return (
    <TableCell className="device-transfer-cell">
      <div className="transfer-chart device-transfer">
        <BarChart
          data={traffic}
          barGap={1}
          barSize={2}
          barCategoryGap={1}
          margin={{ bottom: 0, left: 0, right: 0, top: 0 }}
          width={281}
          height={28}
        >
          <Bar dataKey="upload" width={2} radius={100} fill={ThemeVariable.FgCritical} />
          <Bar dataKey="download" width={2} radius={100} fill={ThemeVariable.FgAction} />
        </BarChart>
      </div>
      <div className="stats-summary">
        <TransferText data={download} variant="download" />
        <span>/</span>
        <TransferText data={upload} variant="upload" />
      </div>
    </TableCell>
  );
};
