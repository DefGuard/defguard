import './style.scss';

import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useAddStandaloneDeviceModal } from '../../store';

export const FinishCliStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.cli.finish;
  const [closeModal] = useAddStandaloneDeviceModal((s) => [s.close], shallow);
  return (
    <div className="finish-cli-step">
      <MessageBox
        type={MessageBoxType.INFO}
        message={localLL.topMessage()}
        dismissId="add-standalone-device-modal-cli-finish-top"
      />
      <div className="download">
        <a href="https://defguard.net/download" target="_blank" rel="noopener noreferrer">
          <Button
            text={localLL.downloadButton()}
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
          />
        </a>
      </div>
      <ExpandableCard
        title={localLL.commandCopy()}
        actions={[
          <ActionButton variant={ActionButtonVariant.COPY} onClick={() => {}} key={0} />,
        ]}
        expanded={true}
        disableExpand={true}
      >
        <p>{'defguard -u https://enrollment.defguard.net -t sdf$&9234&8dfsk345LSD3'}</p>
      </ExpandableCard>
      <div className="controls solo">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.close()}
          onClick={() => {
            closeModal();
          }}
        />
      </div>
    </div>
  );
};
