import { useQuery } from '@tanstack/react-query';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { BadgeProps } from '../../../shared/defguard-ui/components/Badge/types';
import type { IconKindValue } from '../../../shared/defguard-ui/components/Icon/icon-types';
import {
  ThemeVariable,
  type ThemeVariableValue,
} from '../../../shared/defguard-ui/types';

const EXPIRING_THRESHOLD_DAYS = 30;

type WarningSeverity = 'warning' | 'critical' | null;

const getDaysUntil = (value: string | null | undefined): number | null => {
  if (!value) return null;

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;

  const millisecondsPerDay = 1000 * 60 * 60 * 24;
  return Math.ceil((date.getTime() - Date.now()) / millisecondsPerDay);
};

export const useCertificatesWarningState = () => {
  const { data: certsData } = useQuery({
    queryKey: ['core', 'cert', 'certs'],
    queryFn: api.core.getCerts,
    select: (response) => response.data,
  });

  const expiries = [
    certsData?.core_http_cert_source !== 'None' ? certsData?.core_http_cert_expiry : null,
    certsData?.proxy_http_cert_source !== 'None'
      ? certsData?.proxy_http_cert_expiry
      : null,
  ];

  let severity: WarningSeverity = null;
  for (const expiry of expiries) {
    const daysUntil = getDaysUntil(expiry);
    if (daysUntil === null) continue;

    if (daysUntil <= 0) {
      severity = 'critical';
      break;
    }

    if (daysUntil <= EXPIRING_THRESHOLD_DAYS) {
      severity = 'warning';
    }
  }

  const badgeProps: BadgeProps | undefined =
    severity === 'critical'
      ? {
          text: m.settings_certs_warning_expired(),
          variant: 'critical',
        }
      : severity === 'warning'
        ? {
            text: m.settings_certs_warning_expiring(),
            variant: 'warning',
          }
        : undefined;

  const tabIcon: IconKindValue | undefined = severity ? 'attention-filled' : undefined;
  const tabIconColor: ThemeVariableValue | undefined =
    severity === 'critical' ? ThemeVariable.FgCritical : ThemeVariable.FgAttention;

  return {
    severity,
    badgeProps,
    tabIcon,
    tabIconColor,
  };
};
