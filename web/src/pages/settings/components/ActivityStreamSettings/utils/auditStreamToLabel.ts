import { ActivityStream, ActivityStreamType } from '../../../../../shared/types';

export const activityStreamToLabel = (value: ActivityStream): string =>
  activityStreamTypeToLabel(value.stream_type);

export const activityStreamTypeToLabel = (value: ActivityStreamType): string => {
  switch (value) {
    case 'vector_http':
      return 'Vector';
    case 'logstash_http':
      return 'Logstash';
    default:
      return 'Unknown';
  }
};
