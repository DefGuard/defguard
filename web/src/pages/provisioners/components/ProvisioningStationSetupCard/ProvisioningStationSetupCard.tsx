import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { ReactNode, useMemo } from 'react';
import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../../../i18n/i18n-react';
import YubikeyProvisioningGraphic from '../../../../shared/components/svg/YubikeyProvisioningGraphic';
import { ActionButton } from '../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { ExpandableCard } from '../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import useApi from '../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../shared/hooks/useClipboard';
import { QueryKeys } from '../../../../shared/queries';

export const ProvisioningStationSetup = () => {
  const { writeToClipboard } = useClipboard();
  const { LL } = useI18nContext();
  const {
    provisioning: { getWorkerToken },
  } = useApi();

  const { data, isLoading: tokenLoading } = useQuery({
    queryKey: [QueryKeys.FETCH_WORKER_TOKEN],
    queryFn: getWorkerToken,
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  const command = useMemo(
    () =>
      `docker run --privileged ghcr.io/defguard/yubikey-provision:main -t ${data?.token} --id <WORKER_NAME> --grpc <DEFGUARD_GRPC_URL>`,
    [data?.token],
  );

  const tokenActions = useMemo(
    (): ReactNode[] => [
      <ActionButton
        key={0}
        variant={ActionButtonVariant.COPY}
        onClick={() => {
          if (data?.token) {
            void writeToClipboard(
              data.token,
              LL.provisionersOverview.messages.copy.token(),
            );
          }
        }}
      />,
    ],
    [data?.token, LL.provisionersOverview.messages.copy, writeToClipboard],
  );

  const dockerActions = useMemo(
    () => [
      <ActionButton
        key={1}
        variant={ActionButtonVariant.COPY}
        onClick={() => {
          void writeToClipboard(command, LL.provisionersOverview.messages.copy.command());
        }}
      />,
    ],
    [LL.provisionersOverview.messages, command, writeToClipboard],
  );

  return (
    <Card id="provisioning-setup-card">
      <h4>{LL.provisionersOverview.provisioningStation.header()}</h4>
      <p>{LL.provisionersOverview.provisioningStation.content()}</p>
      <div className="image-row">
        <YubikeyProvisioningGraphic />
      </div>
      {data && !isUndefined(data.token) && (
        <ExpandableCard
          disableExpand={true}
          expanded={true}
          title={LL.provisionersOverview.provisioningStation.tokenCard.title()}
          actions={tokenActions}
        >
          <p>{data?.token}</p>
        </ExpandableCard>
      )}
      {data && !isUndefined(data.token) && (
        <ExpandableCard
          title={LL.provisionersOverview.provisioningStation.dockerCard.title()}
          disableExpand={true}
          expanded={true}
          actions={dockerActions}
        >
          <p>{command}</p>
        </ExpandableCard>
      )}
      {tokenLoading && !data && <Skeleton className="command-skeleton" />}
    </Card>
  );
};
