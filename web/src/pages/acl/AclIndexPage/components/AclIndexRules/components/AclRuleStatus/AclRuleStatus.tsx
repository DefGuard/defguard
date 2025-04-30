import './style.scss';

import clsx from 'clsx';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { ActivityIcon } from '../../../../../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../../../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { AclStatus } from '../../../../../types';

type Props = {
  status: AclStatus;
  enabled: boolean;
};

export const AclRuleStatus = ({ enabled, status }: Props) => {
  const { LL } = useI18nContext();
  const statusLL = LL.acl.ruleStatus;

  const [label, iconStatus] = useMemo(() => {
    if (status === AclStatus.APPLIED) {
      switch (enabled) {
        case true:
          return [statusLL.enabled(), ActivityIconVariant.CONNECTED];
        case false:
          return [statusLL.disabled(), ActivityIconVariant.DISCONNECTED];
      }
    }
    switch (status) {
      case AclStatus.DELETED:
        return [statusLL.deleted(), ActivityIconVariant.ERROR];
      case AclStatus.NEW:
        return [statusLL.new(), ActivityIconVariant.CONNECTED];
      case AclStatus.MODIFIED:
        return [statusLL.modified(), ActivityIconVariant.DISCONNECTED];
      case AclStatus.EXPIRED:
        return [statusLL.expired(), ActivityIconVariant.DISABLED];
      default:
        return [statusLL.new(), ActivityIconVariant.CONNECTED];
    }
  }, [enabled, status, statusLL]);

  return (
    <div
      className={clsx('acl-rule-status', `status-${status.valueOf().toLowerCase()}`, {
        disabled: !enabled,
        enabled: enabled,
      })}
    >
      <p>{label}</p>
      <ActivityIcon status={iconStatus} />
    </div>
  );
};
