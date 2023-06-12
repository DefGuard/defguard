import { useQuery } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import { useMemo } from 'react';
import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../i18n/i18n-react';
import {
  ActionButton,
  ActionButtonVariant,
} from '../../shared/components/layout/ActionButton/ActionButton';
import { Card } from '../../shared/components/layout/Card/Card';
import { ExpandableCard } from '../../shared/components/layout/ExpandableCard/ExpandableCard';
import { YubikeyProvisioningGraphic } from '../../shared/components/svg';
import useApi from '../../shared/hooks/useApi';
import { useToaster } from '../../shared/hooks/useToaster';
import { QueryKeys } from '../../shared/queries';

export const ProvisioningStationSetup = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    provisioning: { getWorkerToken },
  } = useApi();

  const { data, isLoading: tokenLoading } = useQuery(
    [QueryKeys.FETCH_WORKER_TOKEN],
    getWorkerToken,
    {
      refetchOnWindowFocus: false,
      refetchOnMount: true,
    }
  );

  const command = useMemo(
    () =>
      `docker run ghcr.io/defguard/yubi-bridge:current --worker-token ${data?.token} --id <WORKER_NAME> --grpc <DEFGUARD_GRPC_URL>`,
    [data?.token]
  );

  const getActions = useMemo(
    () => [
      <ActionButton
        key={1}
        variant={ActionButtonVariant.COPY}
        onClick={() => {
          clipboard
            .write(command)
            .then(() => {
              toaster.success(LL.provisionersOverview.messages.codeCopied());
            })
            .catch((err) => {
              toaster.error(LL.messages.clipboardError());
              console.error(err);
            });
        }}
      />,
    ],
    [LL.messages, LL.provisionersOverview.messages, command, toaster]
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
