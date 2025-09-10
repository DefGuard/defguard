import './style.scss';

import { useMemo } from 'react';
import QRCode from 'react-qr-code';
import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { CopyField } from '../../../../../../../shared/defguard-ui/components/Layout/CopyField/CopyField';
import { MessageBox } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { useClipboard } from '../../../../../../../shared/hooks/useClipboard';
import { enrollmentToImportToken } from '../../../../../../addDevice/utils/enrollmentToToken';
import { useAddUserModal } from '../../hooks/useAddUserModal';

export const EnrollmentTokenCard = () => {
  const { LL } = useI18nContext();
  const tokenResponse = useAddUserModal((state) => state.tokenResponse);
  const { writeToClipboard } = useClipboard();
  const closeModal = useAddUserModal((state) => state.close);
  const isDesktop = useAddUserModal((s) => s.desktop);

  const qrData = useMemo(() => {
    if (tokenResponse) {
      return enrollmentToImportToken(
        tokenResponse.enrollment_url,
        tokenResponse.enrollment_token,
      );
    }
  }, [tokenResponse]);

  if (!isPresent(tokenResponse)) return null;

  return (
    <div id="enrollment-token-step">
      {isDesktop && (
        <MessageBox message={LL.modals.startEnrollment.messageBox.clientForm()} />
      )}
      <CopyField
        label={LL.modals.startEnrollment.urlCard.title()}
        onCopy={writeToClipboard}
        value={tokenResponse.enrollment_url}
      />
      <CopyField
        label={LL.modals.startEnrollment.tokenCard.title()}
        onCopy={writeToClipboard}
        value={tokenResponse.enrollment_token}
      />
      {isPresent(qrData) && isDesktop && (
        <div className="qr">
          <MessageBox message={LL.modals.startEnrollment.messageBox.clientQr()} />
          <QRCode value={qrData} />
        </div>
      )}
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
