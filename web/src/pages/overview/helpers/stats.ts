import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { isUndefined, sortBy, sumBy } from 'lodash-es';

import {
  NetworkDeviceStats,
  NetworkSpeedStats,
  NetworkUserStats,
} from './../../../shared/types';

dayjs.extend(utc);

interface MergeStruct {
  [key: string]: Pick<NetworkSpeedStats, 'download' | 'upload'>;
}

interface UserNetworkSummary {
  summary: NetworkSpeedStats[];
  download: number;
  upload: number;
}

export const summarizeUsersNetworkStats = (
  data: NetworkUserStats[]
): UserNetworkSummary => {
  const merge: MergeStruct = {};
  data.forEach((user) => {
    user.devices.forEach((device) => {
      device.stats.forEach((stat) => {
        const inRank = merge[stat.collected_at];
        if (isUndefined(inRank)) {
          merge[stat.collected_at] = {
            download: stat.download,
            upload: stat.upload,
          };
        } else {
          inRank.download = inRank.download + stat.download;
          inRank.upload = inRank.upload + stat.upload;
        }
      });
    });
  });

  const summary = Object.keys(merge).map((collectedAt) => ({
    collected_at: collectedAt,
    upload: merge[collectedAt].upload,
    download: merge[collectedAt].download,
  }));

  const upload = sumBy(summary, 'upload');
  const download = sumBy(summary, 'download');

  return {
    summary,
    upload,
    download,
  };
};

export const getMaxDeviceStats = (data: NetworkUserStats[]): number => {
  const download: number[] = [];
  data.forEach((obj) =>
    obj.devices.forEach((obj) => obj.stats.forEach((obj) => download.push(obj.download)))
  );

  const upload: number[] = [];
  data.forEach((obj) =>
    obj.devices.forEach((obj) => obj.stats.forEach((obj) => upload.push(obj.upload)))
  );
  const maxDownload = Math.max.apply(null, download);
  const maxUpload = Math.max.apply(null, upload);
  return maxUpload > maxDownload ? maxUpload : maxDownload;
};

export const summarizeDeviceStats = (data: NetworkDeviceStats[]): NetworkSpeedStats[] => {
  const merge: MergeStruct = {};
  data.forEach((device) => {
    device.stats.forEach((stat) => {
      const inRank = merge[stat.collected_at];
      if (isUndefined(inRank)) {
        merge[stat.collected_at] = {
          download: stat.download,
          upload: stat.upload,
        };
      } else {
        inRank.download = inRank.download + stat.download;
        inRank.upload = inRank.upload + stat.upload;
      }
    });
  });
  return Object.keys(merge).map((collectedAt) => ({
    collected_at: collectedAt,
    upload: merge[collectedAt].upload,
    download: merge[collectedAt].download,
  }));
};

export interface StatsChartData extends Pick<NetworkSpeedStats, 'download' | 'upload'> {
  collected_at: number;
}

export const parseStatsForCharts = (data: NetworkSpeedStats[]): StatsChartData[] => {
  const filtered = data.filter((stats) => stats.download > 0 || stats.upload > 0);
  const formatted = filtered.map((stats) => ({
    ...stats,
    collected_at: dayjs.utc(stats.collected_at).toDate().getTime(),
  }));
  return sortBy(formatted, ['collected_at']);
};

/**
 * Helper function for /network/stats/users from param.
 * @param hours how many hours to subtract from current time, this will determinate from how long ago network stats will be aggregated.
 * @returns ISO formatted UTC date as a String value.
 */
export const getNetworkStatsFilterValue = (hours: number): string =>
  dayjs.utc().subtract(hours, 'hours').toISOString();
