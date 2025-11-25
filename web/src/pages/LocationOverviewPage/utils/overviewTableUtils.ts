import type { DeviceStats, TransferStats } from '../../../shared/api/types';
import { mapTransferToChart, type TransferChartData } from '../../../shared/utils/stats';

const mergeStats = (devices: DeviceStats[]): TransferChartData[] => {
  const mergeObject: Record<string, TransferStats> = {};
  devices
    .map((d) => d.stats)
    .forEach((stats) => {
      stats.forEach((s) => {
        const mergeValue = mergeObject[s.collected_at];
        mergeObject[s.collected_at] = {
          collected_at: s.collected_at,
          download: s.download + (mergeValue?.download ?? 0),
          upload: s.upload + (mergeValue?.upload ?? 0),
        };
      });
    });
  return mapTransferToChart(Object.values(mergeObject));
};

export const overviewTableUtils = {
  mergeStats,
};
