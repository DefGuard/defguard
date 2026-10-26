import { useMemo } from 'react';
import { m } from '../../../../../../../../../paraglide/messages';
import api from '../../../../../../../../../shared/api/api';
import {
  AuthenticatorKind,
  type SecurityKey,
} from '../../../../../../../../../shared/api/types';
import { Helper } from '../../../../../../../../../shared/defguard-ui/components/Helper/Helper';
import type { MenuProps } from '../../../../../../../../../shared/defguard-ui/components/Menu/types';
import { Snackbar } from '../../../../../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../../../../../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../../../shared/hooks/modalControls/modalTypes';
import { AuthCardRow } from '../AuthCardRow/AuthCardRow';

type Props = {
  securityKey: SecurityKey;
  username: string;
};

export const WebauthnRow = ({ securityKey, username }: Props) => {
  const menu = useMemo(
    (): MenuProps => ({
      itemGroups: [
        {
          items: [
            {
              text: m.profile_auth_card_delete_passkey(),
              icon: 'delete',
              variant: 'danger',
              onClick: () =>
                openModal(ModalName.ConfirmAction, {
                  title: m.modal_delete_mfa_passkey_title(),
                  contentMd: m.modal_delete_mfa_passkey_content({
                    name: securityKey.name,
                  }),
                  actionPromise: () =>
                    api.auth.mfa.webauthn.deleteKey({ keyId: securityKey.id, username }),
                  invalidateKeys: [['user', username]],
                  submitProps: { text: m.controls_delete(), variant: 'critical' },
                  onSuccess: () => Snackbar.default(m.modal_delete_mfa_passkey_success()),
                  onError: () => Snackbar.error(m.modal_delete_mfa_passkey_error()),
                }),
            },
          ],
        },
      ],
    }),
    [securityKey.id, securityKey.name, username],
  );

  const isHardware = securityKey.authenticator_kind === AuthenticatorKind.Hardware;

  return (
    <AuthCardRow
      icon={isHardware ? 'software-key' : 'key'}
      title={securityKey.name}
      badge={
        isPresent(securityKey.authenticator_name)
          ? {
              text: securityKey.authenticator_name,
              variant: 'outlined',
              size: 'compact',
            }
          : undefined
      }
      menu={menu}
      testId={`passkey-${securityKey.id}`}
    >
      <p>{m.profile_auth_card_passkey_usage_web()}</p>
      {isHardware && (
        <>
          <span className="separator" />
          <p>{m.profile_auth_card_passkey_usage_vpn()}</p>
          <Helper size={16}>{m.profile_auth_card_passkey_usage_vpn_helper()}</Helper>
        </>
      )}
    </AuthCardRow>
  );
};
