import { useQuery } from '@tanstack/react-query';

import { YubikeyProvisioningGraphic } from '../../shared/components/svg';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import KeyBox from '../users/shared/components/KeyBox/KeyBox';

interface Props {
  hasAccess?: boolean;
}

const ProvisioningStationSetup: React.FC<Props> = ({ hasAccess = false }) => {
  const {
    provisioning: { getWorkerToken },
  } = useApi();

  const { data } = useQuery([QueryKeys.FETCH_WORKER_TOKEN], () =>
    getWorkerToken()
  );

  const command = hasAccess
    ? `docker compose run ykdev -g -w ${data?.token}`
    : '';
  return (
    <section
      className={`column provisioning-station-setup ${
        !hasAccess ? 'unavailable' : ''
      }`}
    >
      <div className="content">
        <h4>YubiKey provisioning station</h4>
        <p>
          In order to be able to provision your YubiKeys, first you need to set
          up physical machine with USB slot. Run provided command on your chosen
          machine to register it and start provisioning your keys.
        </p>
        <div className="yubikey-graphic">
          <YubikeyProvisioningGraphic />
        </div>
      </div>
      <KeyBox
        initiallyOpen
        collapsible
        disabled={!hasAccess}
        keyValue={command}
        title="YubiKey provisioning station setup command"
      />
    </section>
  );
};

export default ProvisioningStationSetup;
