import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';

import { InteractionBox } from '../../../../defguard-ui/components/Layout/InteractionBox/InteractionBox';
import useApi from '../../../../hooks/useApi';
import { useToaster } from '../../../../hooks/useToaster';
import { GatewayStatus } from '../../../../types';

type Props = {
  status: GatewayStatus;
};

export const GatewaysFloatingStatus = ({ status }: Props) => {
  const {
    network: { deleteGateway },
  } = useApi();
  const toaster = useToaster();

  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation({
    mutationFn: deleteGateway,
    onError: (err) => {
      toaster.error('Failed to remove gateway');
      console.error(err);
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: ['network', 'gateways'],
      });
      void queryClient.invalidateQueries({
        queryKey: ['network', status.network_id, 'gateways'],
      });
    },
  });

  return (
    <div className="gateway-floating-status-info">
      {status.connected && <IconConnected />}
      {!status.connected && <IconDisconnected />}
      <div className="info">
        {status.name && <p className="name">{status.name}</p>}
        {status.hostname && <p className="hostname">{status.hostname}</p>}
      </div>
      <div className="dismiss">
        {!status.connected && !isPending && (
          <InteractionBox
            onClick={() => {
              mutate({
                gatewayId: status.uid,
                networkId: status.network_id,
              });
            }}
          >
            <IconDismiss />
          </InteractionBox>
        )}
      </div>
    </div>
  );
};

const IconDismiss = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="12"
      height="12"
      viewBox="0 0 22 22"
      fill="none"
    >
      <path
        d="M6.75741 16.6565L16.6569 6.75703C17.0474 6.36651 17.0474 5.73334 16.6569 5.34282C16.2664 4.9523 15.6332 4.9523 15.2427 5.34282L5.3432 15.2423C4.95267 15.6328 4.95267 16.266 5.3432 16.6565C5.73372 17.0471 6.36689 17.0471 6.75741 16.6565Z"
        fill="#899CA8"
      />
      <path
        d="M5.34347 6.75741L15.243 16.6569C15.6335 17.0474 16.2667 17.0474 16.6572 16.6569C17.0477 16.2664 17.0477 15.6332 16.6572 15.2427L6.75769 5.3432C6.36716 4.95267 5.734 4.95267 5.34347 5.3432C4.95295 5.73372 4.95295 6.36689 5.34347 6.75741Z"
        fill="#899CA8"
      />
    </svg>
  );
};

const IconConnected = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="12"
      height="13"
      viewBox="0 0 12 13"
      fill="none"
    >
      <path
        d="M6 12.443C9.31371 12.443 12 9.75673 12 6.44302C12 3.12932 9.31371 0.443024 6 0.443024C2.68629 0.443024 0 3.12932 0 6.44302C0 9.75673 2.68629 12.443 6 12.443Z"
        style={{ fill: 'var(--surface-positive-primary)' }}
      />
      <path
        d="M8.76792 5.50041L5.93949 8.32883C5.79507 8.47325 5.60089 8.53756 5.41215 8.52176C5.2236 8.53737 5.02968 8.47304 4.88542 8.32878L3.4712 6.91457C3.21085 6.65422 3.21085 6.23211 3.4712 5.97176C3.73155 5.71141 4.15366 5.71141 4.41401 5.97176L5.41248 6.97023L7.82511 4.5576C8.08546 4.29725 8.50757 4.29725 8.76792 4.5576C9.02826 4.81795 9.02826 5.24006 8.76792 5.50041Z"
        style={{ fill: 'var(--surface-default-modal)' }}
      />
    </svg>
  );
};

const IconDisconnected = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="12"
      height="13"
      viewBox="0 0 12 13"
      fill="none"
    >
      <path
        d="M6 12.443C9.31371 12.443 12 9.75673 12 6.44302C12 3.12932 9.31371 0.443024 6 0.443024C2.68629 0.443024 0 3.12932 0 6.44302C0 9.75673 2.68629 12.443 6 12.443Z"
        style={{ fill: 'var(--surface-alert-primary)' }}
      />
      <path
        d="M6.72201 7.69602H5.28001L5.05301 2.44302H6.95301L6.72201 7.69602ZM4.99601 9.33502C4.99487 9.21338 5.01905 9.09282 5.06701 8.98102C5.11254 8.8734 5.18076 8.77687 5.26701 8.69802C5.35854 8.61692 5.46433 8.55351 5.57901 8.51102C5.707 8.46377 5.84258 8.44038 5.97901 8.44202C6.11544 8.44038 6.25102 8.46377 6.37901 8.51102C6.4953 8.55235 6.60283 8.6151 6.69601 8.69602C6.78226 8.77487 6.85048 8.8714 6.89601 8.97902C6.94397 9.09082 6.96815 9.21138 6.96701 9.33302C6.96815 9.45467 6.94397 9.57523 6.89601 9.68702C6.85048 9.79465 6.78226 9.89118 6.69601 9.97002C6.60448 10.0511 6.49869 10.1145 6.38401 10.157C6.25602 10.2043 6.12044 10.2277 5.98401 10.226C5.84758 10.2277 5.712 10.2043 5.58401 10.157C5.46933 10.1145 5.36354 10.0511 5.27201 9.97002C5.18576 9.89118 5.11754 9.79465 5.07201 9.68702C5.0226 9.57621 4.99672 9.45635 4.99601 9.33502Z"
        style={{ fill: 'var(--surface-default-modal)' }}
      />
    </svg>
  );
};
