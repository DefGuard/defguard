import './style.scss';

import { Gateway } from '../../../../shared/types';
import { EditButton } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { MutationKeys } from '../../../../shared/mutations';
import useApi from '../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../shared/queries';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { useEditGatewayModal } from '../modals/hooks/useEditGatewayModal';

interface Props {
  gateway: Gateway;
}

export const GatewayCard = ({ gateway }: Props) => {
  const {
    network: {
      gateway: { deleteGateway2 },
    },
  } = useApi();
  const queryClient = useQueryClient();
  const toaster = useToaster();
  const { LL } = useI18nContext();

  const { mutate } = useMutation(
    [MutationKeys.DELETE_GATEWAY],
    deleteGateway2,
    {
      onSuccess: (_data, _variables) => {
        queryClient.invalidateQueries([QueryKeys.FETCH_ALL_GATEWAYS]);
        toaster.success('Gateway removed successfully');
        close();
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const setEditGatewayModal = useEditGatewayModal((state) => state.setState);

  return (
    <div className="gateway-card">
      <div>
        <div>
          <p>
            <strong>ID:</strong> {gateway.id}
          </p>
        </div>
        <div>
          <h3>{gateway.url}</h3>
        </div>
      </div>
      <div>
        <EditButton>
          <EditButtonOption
            text={'Edit gateway'}
            onClick={() => {
              setEditGatewayModal({
                gateway,
                visible: true,
              });
            }}
          />
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text={'Delete gateway'}
            onClick={() => {
              mutate({
                gatewayId: gateway.id,
              });
            }}
          />
        </EditButton>
      </div>
    </div>
  );
};
