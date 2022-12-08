import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { Card } from '../../../shared/components/layout/Card/Card';
import MessageBox, {
  MessageBoxType,
} from '../../../shared/components/layout/MessageBox/MessageBox';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../shared/queries';

export const NetworkGatewaySetup = () => {
  const toaster = useToaster();
  const {
    network: { getGatewayStatus },
  } = useApi();
  const queryClient = useQueryClient();
  const { data: gatewayStatus, isLoading: statusLoading } = useQuery(
    [QueryKeys.FETCH_GATEWAY_STATUS],
    getGatewayStatus,
    {
      onError: (err) => {
        toaster.error('Failed to get gateway status');
        console.error(err);
      },
      refetchOnWindowFocus: false,
    }
  );
  return (
    <section className="gateway">
      <header>
        <h2>Gateway server setup</h2>
      </header>
      <Card>
        <MessageBox>
          <p>
            Please use command below on your gateway server. If you don{"'"}t
            know how, or have some issues please visit our{' '}
            <a>detailed documentation page</a>.
          </p>
        </MessageBox>
        <div className="status">
          <Button
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text="Check connection status"
            loading={statusLoading}
            onClick={() => {
              if (!statusLoading) {
                queryClient.invalidateQueries([QueryKeys.FETCH_GATEWAY_STATUS]);
              }
            }}
          />
          {!gatewayStatus?.connected && !statusLoading && (
            <MessageBox type={MessageBoxType.ERROR}>
              <p>No connection established, please run provided command.</p>
            </MessageBox>
          )}
          {gatewayStatus?.connected && !statusLoading && (
            <MessageBox type={MessageBoxType.SUCCESS}>
              <p>Gateway connected.</p>
            </MessageBox>
          )}
        </div>
      </Card>
    </section>
  );
};
