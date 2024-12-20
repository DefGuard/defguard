import './style.scss';

import { useMemo } from 'react';
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
import { useClipboard } from '../../../../../../shared/hooks/useClipboard';
import { useAddStandaloneDeviceModal } from '../../store';

export const FinishCliStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.cli.finish;
  const [closeModal] = useAddStandaloneDeviceModal((s) => [s.close], shallow);
  const enroll = useAddStandaloneDeviceModal((s) => s.enrollResponse);
  const { writeToClipboard } = useClipboard();

  const commandToCopy = useMemo(() => {
    if (enroll) {
      return `defguard -u ${enroll.enrollment_url} -t ${enroll.enrollment_token}`;
    }
    return '';
  }, [enroll]);

  if (!enroll) return null;
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
            onClick={() => {}}
          />
        </a>
      </div>
      <ExpandableCard
        title={localLL.commandCopy()}
        actions={[
          <ActionButton
            variant={ActionButtonVariant.COPY}
            onClick={() => {
              writeToClipboard(commandToCopy);
            }}
            key={0}
          />,
        ]}
        expanded={true}
        disableExpand={true}
      >
        <p className="config">{commandToCopy}</p>
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
