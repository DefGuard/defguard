import { useQuery } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import { useMemo } from 'react';
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

interface Props {
  hasAccess: boolean;
}

export const ProvisioningStationSetup = ({ hasAccess = false }: Props) => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    provisioning: { getWorkerToken },
  } = useApi();

  const { data } = useQuery([QueryKeys.FETCH_WORKER_TOKEN], getWorkerToken, {
    enabled: hasAccess,
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  const command = hasAccess
    ? `docker compose run ykdev -g -w ${data?.token}`
    : '';

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
    [command, toaster]
  );

  return (
    <Card className="provisioning-setup">
      <h4>{LL.provisionersOverview.provisioningStation.header()}</h4>
      <p>{LL.provisionersOverview.provisioningStation.content()}</p>
      <div className="image-row">
        <YubikeyProvisioningGraphic />
      </div>
      <ExpandableCard
        title={LL.provisionersOverview.provisioningStation.cardTitle()}
        disableExpand={true}
        expanded={true}
        actions={getActions}
      >
        <p>{command}</p>
      </ExpandableCard>
    </Card>
  );
};
