import './style.scss';

import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { IconCheckmarkWhite } from '../../../shared/components/svg';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkControls = () => {
  const navigate = useNavigate();
  const { LL } = useI18nContext();
  const [save, loading] = useNetworkPageStore(
    (state) => [state.saveSubject, state.loading],
    shallow
  );
  return (
    <div className="network-controls">
      <Button
        text={LL.networkConfiguration.form.controls.cancel()}
        size={ButtonSize.SMALL}
        styleVariant={ButtonStyleVariant.LINK}
        onClick={() => navigate('../overview')}
      />
      <Button
        text={LL.networkConfiguration.form.controls.submit()}
        size={ButtonSize.SMALL}
        styleVariant={ButtonStyleVariant.CONFIRM_SUCCESS}
        icon={<IconCheckmarkWhite />}
        loading={loading}
        onClick={() => save.next()}
      />
    </div>
  );
};
