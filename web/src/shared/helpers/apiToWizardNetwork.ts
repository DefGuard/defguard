import { trim } from 'lodash-es';

import { WizardNetwork } from './../../pages/vpn/Wizard/types/types';
import { Network } from './../types';
export const apiToWizardNetwork = (networkData: Network): WizardNetwork => {
  return {
    id: networkData.id,
    name: networkData.name,
    address: networkData.address,
    port: Number(networkData.port),
    endpoint: networkData.endpoint,
    dns:
      trim(networkData?.dns ?? undefined) === ''
        ? undefined
        : networkData?.dns ?? undefined,
    allowedIps:
      trim(networkData?.allowed_ips ?? undefined) === ''
        ? undefined
        : networkData?.allowed_ips ?? undefined,
  };
};
