import { trim } from 'lodash-es';

import { WizardNetwork } from '../../pages/vpn/Wizard/types/types';
import { Network } from '../types';

export const wizardToApiNetwork = (networkData: WizardNetwork): Network => {
  return {
    id: String(networkData.id),
    name: String(networkData.name),
    address: String(networkData.address),
    port: Number(networkData.port),
    endpoint: String(networkData.endpoint),
    dns: trim(networkData.dns) === '' ? null : networkData?.dns ?? null,
    allowed_ips:
      trim(networkData.allowedIps) === ''
        ? null
        : networkData?.allowedIps ?? null,
    connected_at: null,
  };
};
