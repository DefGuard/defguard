import './style.scss';

import { isUndefined } from 'lodash-es';
import { ReactNode, useMemo } from 'react';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { useClipboard } from '../../../../../../../shared/hooks/useClipboard';
import { useAddUserModal } from '../../hooks/useAddUserModal';

export const EnrollmentTokenCard = () => {
  const { LL } = useI18nContext();
  const tokenResponse = useAddUserModal((state) => state.tokenResponse);
  const { writeToClipboard } = useClipboard();
  const closeModal = useAddUserModal((state) => state.close);

  const tokenActions = useMemo(
    (): ReactNode[] => [
      <ActionButton
        data-testid="copy-enrollment-token"
        variant={ActionButtonVariant.COPY}
        disabled={isUndefined(tokenResponse)}
        onClick={() => {
          if (tokenResponse) {
            void writeToClipboard(tokenResponse.enrollment_token);
          }
        }}
        key={0}
      />,
    ],
    [tokenResponse, writeToClipboard],
  );

  const urlActions = useMemo(
    (): ReactNode[] => [
      <ActionButton
        data-testid="copy-enrollment-url"
        variant={ActionButtonVariant.COPY}
        disabled={!tokenResponse}
        onClick={() => {
          if (tokenResponse) {
            void writeToClipboard(tokenResponse.enrollment_url);
          }
        }}
        key={0}
      />,
    ],
    [tokenResponse, writeToClipboard],
  );

  return (
    <div id="enrollment-token-step">
      <ExpandableCard
        title={LL.modals.startEnrollment.urlCard.title()}
        actions={urlActions}
        expanded
      >
        <p>{tokenResponse?.enrollment_url}</p>
      </ExpandableCard>
      <ExpandableCard
        title={LL.modals.startEnrollment.tokenCard.title()}
        actions={tokenActions}
        expanded
      >
        <p>{tokenResponse?.enrollment_token}</p>
      </ExpandableCard>
      <div className="controls">
        <Button
          type="button"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.form.close()}
          className="cancel"
          onClick={() => closeModal()}
        />
      </div>
    </div>
  );
};
