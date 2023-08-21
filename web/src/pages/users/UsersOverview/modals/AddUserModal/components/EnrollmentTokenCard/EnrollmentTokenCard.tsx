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

  const actions = useMemo(
    (): ReactNode[] => [
      <ActionButton
        variant={ActionButtonVariant.COPY}
        disabled={isUndefined(tokenResponse)}
        onClick={() => {
          if (tokenResponse) {
            const res = `URL: ${tokenResponse.enrollment_url} \nToken: ${tokenResponse.enrollment_token}`;
            writeToClipboard(res);
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
        title={LL.modals.startEnrollment.tokenCard.title()}
        actions={actions}
        expanded
        disableExpand
      >
        <p>URL: {tokenResponse?.enrollment_url}</p>
        <p>TOKEN: {tokenResponse?.enrollment_token}</p>
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
