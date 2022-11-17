import './style.scss';

import { useNavigate } from 'react-router';
import shallow from 'zustand/shallow';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { IconCheckmarkWhite } from '../../shared/components/svg';
import { useNetworkPageStore } from './hooks/useNetworkPageStore';
import { NetworkConfiguration } from './NetworkConfiguration/NetworkConfiguration';
import { NetworkGatewaySetup } from './NetworkGateway/NetworkGateway';

export const NetworkPage = () => {
  const navigate = useNavigate();
  return (
    <PageContainer id="network-page">
      <header>
        <h1>Edit Network</h1>
        <div className="controls">
          <Button
            text="Back"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.LINK}
            onClick={() => navigate('../overview')}
          />
          <SaveFormButton />
        </div>
      </header>
      <NetworkConfiguration />
      <NetworkGatewaySetup />
    </PageContainer>
  );
};

const SaveFormButton = () => {
  const [formValid, save, loading] = useNetworkPageStore(
    (state) => [state.formValid, state.saveSubject, state.loading],
    shallow
  );
  return (
    <Button
      text="Save changes"
      size={ButtonSize.SMALL}
      styleVariant={ButtonStyleVariant.CONFIRM_SUCCESS}
      icon={<IconCheckmarkWhite />}
      disabled={!formValid}
      loading={loading}
      onClick={() => save.next()}
    />
  );
};
