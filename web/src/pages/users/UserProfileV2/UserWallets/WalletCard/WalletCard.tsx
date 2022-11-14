import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import { AvatarBox } from '../../../../../shared/components/layout/AvatarBox/AvatarBox';
import Badge, {
  BadgeStyleVariant,
} from '../../../../../shared/components/layout/Badge/Badge';
import { Card } from '../../../../../shared/components/layout/Card/Card';
import { EditButton } from '../../../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../../../shared/components/layout/EditButton/EditButtonOption';
import { Label } from '../../../../../shared/components/layout/Label/Label';
import { IconEth } from '../../../../../shared/components/svg';
import { useUserProfileV2Store } from '../../../../../shared/hooks/store/useUserProfileV2Store';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';
import { WalletInfo } from '../../../../../shared/types';

interface Props {
  wallet: WalletInfo;
  connected?: boolean;
  showMFA?: boolean;
}

export const WalletCard = ({
  wallet,
  connected = false,
  showMFA = false,
}: Props) => {
  const toaster = useToaster();
  const [hovered, setHovered] = useState(false);
  const {
    user: { deleteWallet },
    auth: {
      mfa: {
        web3: { updateWalletMFA },
      },
    },
  } = useApi();
  const queryClient = useQueryClient();
  const user = useUserProfileV2Store((state) => state.user);

  const { mutate: deleteWalletMutation } = useMutation(
    [MutationKeys.DELETE_WALLET],
    deleteWallet,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        toaster.success('Wallet deleted');
      },
      onError: () => {
        toaster.error('Wallet deletion failed');
      },
    }
  );

  const { mutate: updateWalletMFAMutation } = useMutation(
    [MutationKeys.EDIT_WALLET_MFA],
    updateWalletMFA,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
        toaster.success('Wallet MFA changed');
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Wallet change failed');
      },
    }
  );

  return (
    <Card
      className="wallet-card"
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
    >
      <EditButton visible={hovered}>
        {!wallet.use_for_mfa && showMFA && (
          <EditButtonOption
            text="Enable MFA"
            onClick={() => {
              if (user) {
                updateWalletMFAMutation({
                  username: user.username,
                  address: wallet.address,
                  use_for_mfa: true,
                });
              }
            }}
          />
        )}
        {wallet.use_for_mfa && showMFA && (
          <EditButtonOption
            text="Disable MFA"
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            onClick={() => {
              console.log(user);
              if (user) {
                updateWalletMFAMutation({
                  username: user.username,
                  address: wallet.address,
                  use_for_mfa: false,
                });
              }
            }}
          />
        )}
        <EditButtonOption
          text="Delete"
          styleVariant={EditButtonOptionStyleVariant.WARNING}
          onClick={() => {
            if (user) {
              deleteWalletMutation({
                username: user.username,
                address: wallet.address,
                chainId: wallet.chain_id,
                name: wallet.name,
              });
            }
          }}
        />
      </EditButton>
      <div className="top">
        <AvatarBox>
          <IconEth />
        </AvatarBox>
        <h3 data-test="wallet-name">{wallet.name}</h3>
        {connected && (
          <Badge text="Connected" styleVariant={BadgeStyleVariant.STANDARD} />
        )}
        {wallet.use_for_mfa && (
          <Badge text="MFA" styleVariant={BadgeStyleVariant.STANDARD} />
        )}
      </div>
      <div className="bottom">
        <Label>Address</Label>
        <p data-test="wallet-address">{wallet.address}</p>
      </div>
    </Card>
  );
};
