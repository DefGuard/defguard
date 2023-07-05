import './style.scss';

import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { Select, SelectOption } from '../../../shared/components/layout/Select/Select';
import { IconCheckmarkWhite } from '../../../shared/components/svg';
import { deviceBreakpoints } from '../../../shared/constants';
import { useWizardStore } from '../../wizard/hooks/useWizardStore';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkControls = () => {
  const navigate = useNavigate();
  const resetWizardState = useWizardStore((state) => state.resetState);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const { LL } = useI18nContext();
  const [save, setNetworkState] = useNetworkPageStore(
    (state) => [state.saveSubject, state.setState],
    shallow
  );
  const [loading, selectedNetworkId] = useNetworkPageStore(
    (state) => [state.loading, state.selectedNetworkId],
    shallow
  );
  const networks = useNetworkPageStore((state) => state.networks);

  const getOptions = useMemo(
    (): SelectOption<number>[] =>
      networks.map((n) => ({
        value: n.id,
        label: n.name,
        key: n.id,
      })),
    [networks]
  );

  const selectedOption = useMemo(
    () => getOptions.find((o) => o.value === selectedNetworkId),
    [getOptions, selectedNetworkId]
  );

  return (
    <div className="network-controls">
      {breakpoint !== 'desktop' && (
        <div className="network-select">
          <Select
            selected={selectedOption}
            options={getOptions}
            addOptionLabel={LL.networkPage.addNetwork()}
            outerLabel={LL.networkPage.controls.networkSelect.label()}
            onChange={(res) => {
              if (!Array.isArray(res) && res) {
                setNetworkState({ selectedNetworkId: res.value });
              }
            }}
            onCreate={() => {
              resetWizardState();
              navigate('/admin/wizard', { replace: true });
            }}
          />
        </div>
      )}
      <Button
        className="cancel"
        text={LL.networkConfiguration.form.controls.cancel()}
        size={ButtonSize.SMALL}
        styleVariant={ButtonStyleVariant.LINK}
        onClick={() => navigate('/admin/overview', { replace: true })}
      />
      <Button
        className="submit"
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
