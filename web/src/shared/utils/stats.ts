import dayjs from 'dayjs';
import { orderBy } from 'lodash-es';
import type { TransferStats } from '../api/types';

export interface TransferChartData {
  download: number;
  upload: number;
  timestamp: number;
}

export const mapTransferToChart = (stats: TransferStats[]): TransferChartData[] => {
  // filter out blanks so visually chart is more dense as empty ticks take space
  const distilled = stats.filter((tick) => tick.download || tick.upload);
  const res = distilled.map(
    ({ download, upload, collected_at }): TransferChartData => ({
      download,
      upload,
      // always display date in local
      timestamp: dayjs.utc(collected_at).local().unix(),
    }),
  );
  return orderBy(res, ['timestamp'], ['asc']);
};
