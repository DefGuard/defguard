import { useQuery } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import { useMemo } from 'react';

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
              toaster.success('Command copied.');
            })
            .catch((err) => {
              toaster.error('Clipboard is not accessible.');
              console.error(err);
            });
        }}
      />,
    ],
    [command, toaster]
  );

  return (
    <Card className="provisioning-setup">
      <h4>YubiKey provisioning station</h4>
      <p>
        In order to be able to provision your YubiKeys, first you need to set up
        physical machine with USB slot. Run provided command on your chosen
        machine to register it and start provisioning your keys.
      </p>
      <YubikeyProvisioningGraphic />
      <ExpandableCard
        title="Provisioning station setup command"
        disableExpand={true}
        expanded={true}
        actions={getActions}
      >
        <p>{command}</p>
      </ExpandableCard>
    </Card>
  );
};
