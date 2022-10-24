import './style.scss';

import { AvatarBox } from '../../../../../shared/components/layout/AvatarBox/AvatarBox';
import { Card } from '../../../../../shared/components/layout/Card/Card';
import { Label } from '../../../../../shared/components/layout/Label/Label';
import { WalletInfo } from '../../../../../shared/types';

interface Props {
  wallet: WalletInfo;
}

export const WalletCard = ({ wallet }: Props) => {
  return (
    <Card className="wallet-card">
      <div className="top">
        <AvatarBox></AvatarBox>
        <span data-test="wallet-name">{wallet.name}</span>
      </div>
      <div className="bottom">
        <Label>Address</Label>
        <p data-test="wallet-address">{wallet.address}</p>
      </div>
    </Card>
  );
};
