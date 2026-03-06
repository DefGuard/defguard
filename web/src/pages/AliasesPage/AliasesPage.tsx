import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Suspense, useMemo, useState } from 'react';
import { AclDeploymentState, type AclDeploymentStateValue } from '../../shared/api/types';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getAliasesCountQueryOptions } from '../../shared/query';
import { DeleteAliasDestinationConfirmModal } from '../Acl/components/DeleteAliasDestinationConfirmModal/DeleteAliasDestinationConfirmModal';
import { AliasesDeployedTab } from './tabs/AliasesDeployedTab';
import { AliasesPendingTab } from './tabs/AliasesPendingTab';

export const AliasesPage = () => {
  const { data: aliasesCount } = useQuery(getAliasesCountQueryOptions);

  const [activeTab, setActiveTab] = useState<AclDeploymentStateValue>(
    AclDeploymentState.Applied,
  );

  const pendingCount = aliasesCount?.pending ?? 0;
  const pendingTitle = pendingCount ? `Pending (${pendingCount})` : 'Pending';
  const pendingIcon = pendingCount > 0 ? IconKind.AttentionFilled : undefined;

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        active: activeTab === AclDeploymentState.Applied,
        onClick: () => {
          setActiveTab(AclDeploymentState.Applied);
        },
        title: 'Deployed',
      },
      {
        active: activeTab === AclDeploymentState.Modified,
        onClick: () => {
          setActiveTab(AclDeploymentState.Modified);
        },
        title: pendingTitle,
        icon: pendingIcon,
      },
    ],
    [activeTab, pendingIcon, pendingTitle],
  );

  return (
    <Page id="aliases-page" title={'Aliases'}>
      <TablePageLayout>
        <Tabs items={tabs} />
        <Suspense fallback={<TableSkeleton />}>
          {activeTab === AclDeploymentState.Applied && <AliasesDeployedTab />}
          {activeTab === AclDeploymentState.Modified && <AliasesPendingTab />}
        </Suspense>
        <DeleteAliasDestinationConfirmModal />
      </TablePageLayout>
    </Page>
  );
};
