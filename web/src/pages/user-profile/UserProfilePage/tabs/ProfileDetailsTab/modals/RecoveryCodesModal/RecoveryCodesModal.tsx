import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import { CopyButton } from '../../../../../../../shared/components/CopyButton/CopyButton';
import { DownloadButton } from '../../../../../../../shared/components/DownloadButton/DownloadButton';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import { subscribeOpenModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import type { ModalNameValue } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { useAuth } from '../../../../../../../shared/hooks/useAuth';
import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { downloadText } from '../../../../../../../shared/utils/download';

const modalNameKey: ModalNameValue = 'recoveryCodes' as const;

export const RecoveryCodesModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [recoveryCodes, setRecoveryCodes] = useState<string[]>([]);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (codes) => {
      setOpen(true);
      setRecoveryCodes(codes);
    });
    return () => {
      openSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="recovery-codes-modal"
      size="small"
      title={m.modal_recovery_codes_download_title()}
      isOpen={isOpen}
    >
      <ModalContent codes={recoveryCodes} />
    </Modal>
  );
};

const ModalContent = ({ codes }: { codes: string[] }) => {
  const [confirmed, setConfirmed] = useState(false);
  const [confirmError, setConfirmError] = useState(false);
  const navigate = useNavigate();
  const username = useAuth((s) => s.user?.username);

  const { mutate, isPending } = useMutation({
    mutationFn: api.auth.mfa.enable,
    meta: {
      invalidate: username
        ? [['session-info'], ['me'], ['user', username]]
        : [['session-info'], ['me']],
    },
    onSuccess: () => {
      useAuth.getState().setUser();
      navigate({
        to: '/auth/login',
        replace: true,
      });
    },
  });

  const codesSplit = useMemo(() => {
    return [codes.slice(0, 4), codes.slice(4, 8)];
  }, [codes]);

  return (
    <>
      <p className="explain">{m.modal_recovery_codes_explain()}</p>
      <SizedBox height={ThemeSpacing.Xl2} />
      <div className="codes">
        {codesSplit.map((col, index) => (
          <div className="col" key={index}>
            {col.map((code) => (
              <p className="code" key={code}>
                {code}
              </p>
            ))}
          </div>
        ))}
      </div>
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="actions">
        <DownloadButton
          size="big"
          text={m.modal_recovery_codes_download_cta_download()}
          onDownload={() => {
            const fileContent = codes.join('\n');
            downloadText(fileContent, 'recovery', 'txt');
          }}
        />
        <CopyButton value={codes.join('\n')} size="big" testId="copy-recovery-codes" />
      </div>
      <SizedBox height={ThemeSpacing.Xl2} />
      <div className="bottom">
        <Checkbox
          active={confirmed}
          text={m.modal_recovery_codes_download_confirm()}
          testId="confirm-code-save"
          error={confirmError ? m.modal_recovery_codes_error() : undefined}
          onClick={() => {
            setConfirmed((s) => !s);
            setConfirmError(false);
          }}
        />
        <Button
          variant="primary"
          text={m.controls_complete()}
          testId="finish-recovery-codes"
          loading={isPending}
          onClick={() => {
            if (!confirmed) {
              setConfirmError(true);
            }
            if (confirmed) {
              mutate();
            }
          }}
        />
      </div>
    </>
  );
};
