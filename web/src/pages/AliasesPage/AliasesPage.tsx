import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import api from '../../shared/api/api';
import {
  AclAliasStatus,
  AclDeploymentState,
  type AclDeploymentStateValue,
} from '../../shared/api/types';
import { Page } from '../../shared/components/Page/Page';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { AliasesDeployedTab } from './tabs/AliasesDeployedTab';
import { AliasesPendingTab } from './tabs/AliasesPendingTab';

export const AliasesPage = () => {
  const [activeTab, setActiveTab] = useState<AclDeploymentStateValue>(
    AclDeploymentState.Applied,
  );

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
        title: 'Pending',
      },
    ],
    [activeTab],
  );

  const { data: aliases } = useQuery({
    queryFn: api.acl.alias.getAliases,
    queryKey: ['acl', 'alias'],
    select: (resp) => resp.data,
  });

  const deployedAliases = useMemo(() => {
    if (isPresent(aliases)) {
      return aliases.filter((alias) => alias.state === AclAliasStatus.Applied);
    }
  }, [aliases]);

  const pendingAliases = useMemo(() => {
    if (isPresent(aliases)) {
      return aliases.filter((alias) => alias.state === AclAliasStatus.Modified);
    }
  }, [aliases]);

  return (
    <Page id="aliases-page" title={'Aliases'}>
      <Tabs items={tabs} />
      {isPresent(deployedAliases) && activeTab === AclDeploymentState.Applied && (
        <AliasesDeployedTab aliases={deployedAliases} />
      )}
      {isPresent(pendingAliases) && activeTab === AclDeploymentState.Modified && (
        <AliasesPendingTab aliases={pendingAliases} />
      )}
    </Page>
  );
};
