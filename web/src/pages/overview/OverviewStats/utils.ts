import { groupBy, map, sortBy } from 'lodash-es';

import { NetworkSpeedStats } from '../../../shared/types';

type AggregatedTick = {
  collected_at: string;
  download: number;
  upload: number;
  count: number;
};

export const networkTrafficToChartData = (
  ticks: NetworkSpeedStats[],
  filter: number,
): NetworkSpeedStats[] => {
  if (filter >= 2 && filter <= 5 && ticks.length > 60) {
    const sorted = sortBy(ticks, (tick) => new Date(tick.collected_at).getTime());
    const first = new Date(sorted[0].collected_at).getTime();
    const last = new Date(sorted[sorted.length - 1].collected_at).getTime();

    const totalMinutes = Math.max(1, Math.floor((last - first) / (1000 * 60)));
    const minutesPerBucket = Math.ceil(totalMinutes / 60);

    const grouped = groupBy(sorted, (tick) => {
      const date = new Date(tick.collected_at);
      const bucketTime = new Date(
        Math.floor(date.getTime() / (minutesPerBucket * 60 * 1000)) *
          minutesPerBucket *
          60 *
          1000,
      );
      return bucketTime.toISOString();
    });

    return map(grouped, (group, timestamp): AggregatedTick => {
      const totalDownload = group.reduce((sum, t) => sum + t.download, 0);
      const totalUpload = group.reduce((sum, t) => sum + t.upload, 0);
      const count = group.length;

      return {
        collected_at: timestamp,
        download: totalDownload,
        upload: totalUpload,
        count,
      };
    });
  }
  return ticks;
};
