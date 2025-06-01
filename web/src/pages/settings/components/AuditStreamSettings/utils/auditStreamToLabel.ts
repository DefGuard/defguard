import { AuditStream, AuditStreamType } from '../../../../../shared/types';

export const auditStreamToLabel = (value: AuditStream): string =>
  auditStreamTypeToLabel(value.stream_type);

export const auditStreamTypeToLabel = (value: AuditStreamType): string => {
  switch (value) {
    case 'vector_http':
      return 'Vector';
    case 'logstash_http':
      return 'Logstash';
    default:
      return 'Unknown';
  }
};
