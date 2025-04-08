import './style.scss';

import clsx from 'clsx';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { ActivityIcon } from '../../../../../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../../../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { AclAliasStatus } from '../../../../../types';

type Props = {
  status: AclAliasStatus;
};

export const AclAliasStatusDisplay = ({ status }: Props) => {
  const { LL } = useI18nContext();
  const statusLL = LL.acl.listPage.aliases.list.status;

  const [label, iconStatus] = useMemo(() => {
    switch (status) {
      case AclAliasStatus.APPLIED:
        return [statusLL.applied(), ActivityIconVariant.CONNECTED];
      case AclAliasStatus.MODIFIED:
        return [statusLL.changed(), ActivityIconVariant.DISCONNECTED];
    }
  }, [status, statusLL]);

  return (
    <div className={clsx('acl-alias-status', `status-${status.valueOf().toLowerCase()}`)}>
      <p>{label}</p>
      <ActivityIcon status={iconStatus} />
    </div>
  );
};
