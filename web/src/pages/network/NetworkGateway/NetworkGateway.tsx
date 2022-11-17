import './style.scss';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { Card } from '../../../shared/components/layout/Card/Card';
import MessageBox, {
  MessageBoxType,
} from '../../../shared/components/layout/MessageBox/MessageBox';

export const NetworkGatewaySetup = () => {
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
          />
          <MessageBox type={MessageBoxType.ERROR}>
            <p>No connection established, please run provided command.</p>
          </MessageBox>
          {/**
          <MessageBox type={MessageBoxType.SUCCESS}>
            <p>Gateway connected.</p>
          </MessageBox>
            **/}
        </div>
      </Card>
    </section>
  );
};
