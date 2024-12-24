import './style.scss';

import { useMemo } from 'react';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useClipboard } from '../../../../../shared/hooks/useClipboard';
import { externalLink } from '../../../../../shared/links';
import { StartEnrollmentResponse } from '../../../../../shared/types';

type Props = {
  enrollmentData: StartEnrollmentResponse;
};
export const StandaloneDeviceModalEnrollmentContent = ({
  enrollmentData: { enrollment_token, enrollment_url },
}: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.components.standaloneDeviceTokenModalContent;
  const { writeToClipboard } = useClipboard();
  const commandToCopy = useMemo(() => {
    return `defguard -u ${enrollment_url} -t ${enrollment_token}`;
  }, [enrollment_token, enrollment_url]);

  return (
    <div className="standalone-device-enrollment-content">
      <MessageBox
        type={MessageBoxType.INFO}
        message={localLL.headerMessage()}
        dismissId="standalone-device-enrollment-modal-content-header"
      />
      <div className="download">
        <a
          href={externalLink.defguardCliDownload}
          target="_blank"
          rel="noopener noreferrer"
        >
          <Button
            text={localLL.downloadButton()}
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            onClick={() => {}}
          />
        </a>
      </div>
      <ExpandableCard
        title={localLL.expandableCard.title()}
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
        <p className="code">{commandToCopy}</p>
      </ExpandableCard>
    </div>
  );
};
