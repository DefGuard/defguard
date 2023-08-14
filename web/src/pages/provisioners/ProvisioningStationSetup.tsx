import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../i18n/i18n-react';
import YubikeyProvisioningGraphic from '../../shared/components/svg/YubikeyProvisioningGraphic';
import { ActionButton } from '../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { ExpandableCard } from '../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import useApi from '../../shared/hooks/useApi';
import { useClipboard } from '../../shared/hooks/useClipboard';
import { QueryKeys } from '../../shared/queries';

export const ProvisioningStationSetup = () => {
  const { writeToClipboard } = useClipboard();
  const { LL } = useI18nContext();
  const {
    provisioning: { getWorkerToken },
  } = useApi();

  const { data, isLoading: tokenLoading } = useQuery(
    [QueryKeys.FETCH_WORKER_TOKEN],
    getWorkerToken,
    {
      refetchOnWindowFocus: false,
      refetchOnMount: true,
    },
  );

  const command = useMemo(
    () =>
      `docker run ghcr.io/defguard/yubi-bridge:current --worker-token ${data?.token} --id <WORKER_NAME> --grpc <DEFGUARD_GRPC_URL>`,
    [data?.token],
  );

  const getActions = useMemo(
    () => [
      <ActionButton
        key={1}
        variant={ActionButtonVariant.COPY}
        onClick={() => {
          writeToClipboard(command, LL.provisionersOverview.messages.codeCopied());
        }}
      />,
    ],
    [LL.provisionersOverview.messages, command, writeToClipboard],
  );

  return (
    <Card className="provisioning-setup">
      <h4>{LL.provisionersOverview.provisioningStation.header()}</h4>
      <p>{LL.provisionersOverview.provisioningStation.content()}</p>
      <div className="image-row">
        <YubikeyProvisioningGraphic />
      </div>
      {!tokenLoading && (
        <ExpandableCard
          title={LL.provisionersOverview.provisioningStation.cardTitle()}
          disableExpand={true}
          expanded={true}
          actions={getActions}
        >
          <p>{command}</p>
        </ExpandableCard>
      )}
      {tokenLoading && <Skeleton className="command-skeleton" />}
    </Card>
  );
};
