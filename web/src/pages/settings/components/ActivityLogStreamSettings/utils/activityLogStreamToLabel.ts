import { ActivityLogStream, ActivityLogStreamType } from '../../../../../shared/types';

export const activityLogStreamToLabel = (value: ActivityLogStream): string =>
  activityLogStreamTypeToLabel(value.stream_type);

export const activityLogStreamTypeToLabel = (value: ActivityLogStreamType): string => {
  switch (value) {
    case 'vector_http':
      return 'Vector';
    case 'logstash_http':
      return 'Logstash';
    default:
      return 'Unknown';
  }
};
