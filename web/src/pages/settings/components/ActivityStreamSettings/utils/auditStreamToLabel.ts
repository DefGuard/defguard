import { ActivityStream, ActivityStreamType } from '../../../../../shared/types';

export const auditStreamToLabel = (value: ActivityStream): string =>
  auditStreamTypeToLabel(value.stream_type);

export const auditStreamTypeToLabel = (value: ActivityStreamType): string => {
  switch (value) {
    case 'vector_http':
      return 'Vector';
    case 'logstash_http':
      return 'Logstash';
    default:
      return 'Unknown';
  }
};
