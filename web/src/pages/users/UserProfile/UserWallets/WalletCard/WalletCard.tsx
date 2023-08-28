import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconEth from '../../../../../shared/components/svg/IconEth';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { AvatarBox } from '../../../../../shared/defguard-ui/components/Layout/AvatarBox/AvatarBox';
import { Badge } from '../../../../../shared/defguard-ui/components/Layout/Badge/Badge';
import { BadgeStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/Badge/types';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../../shared/hooks/useClipboard';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';
import { WalletInfo } from '../../../../../shared/types';

interface Props {
  wallet: WalletInfo;
  connected?: boolean;
  showMFA?: boolean;
}

export const WalletCard = ({ wallet, connected = false, showMFA = false }: Props) => {
  const { writeToClipboard } = useClipboard();
  const { LL } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const setModalsState = useModalStore((state) => state.setState);
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
  const user = useUserProfileStore((state) => state.userProfile?.user);

  const { mutate: deleteWalletMutation } = useMutation(
    [MutationKeys.DELETE_WALLET],
    deleteWallet,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        toaster.success(LL.userPage.wallets.card.messages.deleteSuccess());
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const { mutate: updateWalletMFAMutation } = useMutation(
    [MutationKeys.EDIT_WALLET_MFA],
    updateWalletMFA,
    {
      onSuccess: (data, props) => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        if (props.use_for_mfa) {
          toaster.success(LL.userPage.wallets.card.messages.enableMFA());
        } else {
          toaster.success(LL.userPage.wallets.card.messages.disableMFA());
        }
        if (data && data.codes) {
          setModalsState({
            recoveryCodesModal: { visible: true, codes: data.codes },
          });
        }
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const copyWalletAddress = useCallback(() => {
    writeToClipboard(wallet.address, LL.userPage.wallets.messages.addressCopied());
  }, [LL.userPage.wallets.messages, wallet.address, writeToClipboard]);

  return (
    <Card
      className="wallet-card"
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
    >
      <EditButton visible={hovered || breakpoint !== 'desktop'}>
        <EditButtonOption
          text={LL.userPage.wallets.card.edit.copyAddress()}
          onClick={copyWalletAddress}
        />
        {!wallet.use_for_mfa && showMFA && (
          <EditButtonOption
            text={LL.userPage.wallets.card.edit.enableMFA()}
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
            text={LL.userPage.wallets.card.edit.disableMFA()}
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            onClick={() => {
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
          text={LL.userPage.wallets.card.edit.delete()}
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
        <h3 data-testid="wallet-name">{wallet.name}</h3>
        {connected && (
          <Badge text="Connected" styleVariant={BadgeStyleVariant.STANDARD} />
        )}
        {wallet.use_for_mfa && (
          <Badge
            text={LL.userPage.wallets.card.mfaBadge()}
            styleVariant={BadgeStyleVariant.STANDARD}
          />
        )}
      </div>
      <div className="bottom">
        <Label>{LL.userPage.wallets.card.address()}</Label>
        <p data-testid="wallet-address">{wallet.address}</p>
      </div>
    </Card>
  );
};
