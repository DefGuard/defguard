import classNames from 'classnames';
import { saveAs } from 'file-saver';
import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import SvgIconClip from '../../../../../../shared/components/svg/IconClip';
import SvgIconCollapse from '../../../../../../shared/components/svg/IconCollapse';
import SvgIconExpand from '../../../../../../shared/components/svg/IconExpand';
import { DeviceAvatar } from '../../../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { Label } from '../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { TextContainer } from '../../../../../../shared/defguard-ui/components/Layout/TextContainer/TextContainer';
import { useUserProfileStore } from '../../../../../../shared/hooks/store/useUserProfileStore';
import { useClipboard } from '../../../../../../shared/hooks/useClipboard';
import { AuthenticationKey, AuthenticationKeyType } from '../../../../../../shared/types';
import { useRenameAuthenticationKeyModal } from '../../../../shared/modals/RenameAuthenticationKeyModal/useRenameAuthenticationKeyModal';
import { useDeleteAuthenticationKeyModal } from '../../DeleteAuthenticationKeyModal/useDeleteAuthenticationKeyModal';

type Props = {
  yubikey: YubikeyData;
  keys: AuthenticationKey[];
};

type YubikeyData = {
  yubikey_name: string;
  yubikey_id: number;
  yubikey_serial: string;
};

export const AuthenticationKeyItemYubikey = ({ yubikey, keys }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.userPage.authenticationKeys.keysList;
  const [hovered, setHovered] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const openRenameModal = useRenameAuthenticationKeyModal((s) => s.open);
  const username = useUserProfileStore((s) => s.userProfile?.user.username);
  const { writeToClipboard } = useClipboard();
  const openDeleteModal = useDeleteAuthenticationKeyModal((s) => s.open);

  const cn = useMemo(
    () =>
      classNames('authentication-key-item', 'yubikey', {
        expanded: expanded,
        active: expanded || hovered,
      }),
    [expanded, hovered],
  );

  const downloadKey = useCallback(
    (keyType: AuthenticationKeyType) => {
      const key = keys.find((k) => k.key_type === keyType);
      if (key) {
        const data = new Blob([key.key], { type: 'text/plain;charset=utf-8' });
        saveAs(
          data,
          `${yubikey.yubikey_name.replace(' ', '').toLocaleLowerCase()}_${keyType.valueOf().toLowerCase()}.pub`,
        );
      }
    },
    [keys, yubikey.yubikey_name],
  );

  const copyKey = useCallback(
    (keyType: AuthenticationKeyType) => {
      const key = keys.find((k) => k.key_type === keyType);
      if (key) {
        void writeToClipboard(key.key);
      }
    },
    [keys, writeToClipboard],
  );

  return (
    <div
      className={cn}
      onMouseOver={() => setHovered(true)}
      onMouseOut={() => setHovered(false)}
    >
      <header className="item-content">
        <div className="top">
          <DeviceAvatar deviceId={yubikey.yubikey_id} active={false} />
          <TextContainer text={yubikey.yubikey_name} />
        </div>
        <div className="bottom">
          <Label>{localLL.common.serialNumber()}</Label>
          <TextContainer text={yubikey.yubikey_serial} />
        </div>
        <div className="controls">
          <EditButton>
            <EditButtonOption
              text={`${localLL.common.copy()} GPG ${localLL.common.key()}`}
              onClick={() => copyKey(AuthenticationKeyType.GPG)}
            />
            <EditButtonOption
              text={`${localLL.common.copy()} SSH ${localLL.common.key()}`}
              onClick={() => copyKey(AuthenticationKeyType.SSH)}
            />
            <EditButtonOption
              text={`${localLL.common.download()} GPG ${localLL.common.key()}`}
              onClick={() => downloadKey(AuthenticationKeyType.GPG)}
            />
            <EditButtonOption
              text={`${localLL.common.download()} SSH ${localLL.common.key()}`}
              onClick={() => downloadKey(AuthenticationKeyType.SSH)}
            />
            <EditButtonOption
              text={`${localLL.common.rename()} YubiKey`}
              onClick={() => {
                if (username) {
                  openRenameModal({
                    id: yubikey.yubikey_id,
                    key_type: 'yubikey',
                    name: yubikey.yubikey_name,
                    username: username,
                  });
                }
              }}
            />
            <EditButtonOption
              text={`${localLL.common.delete()} YubiKey`}
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              onClick={() => {
                if (username) {
                  openDeleteModal({
                    id: yubikey.yubikey_id,
                    name: yubikey.yubikey_name,
                    type: 'yubikey',
                    username,
                  });
                }
              }}
            />
          </EditButton>
          <button className="expand-button" onClick={() => setExpanded((s) => !s)}>
            {expanded ? <SvgIconCollapse /> : <SvgIconExpand />}
          </button>
        </div>
      </header>
      <div className="expandable-section">
        <div>
          {keys.map((key) => (
            <div key={key.id} className="item-content">
              <div className="top">
                <SvgIconClip />
                <p>{`${key.key_type.valueOf().toUpperCase()} Key`}</p>
              </div>
              <div className="bottom">
                <Label>{localLL.common.key()}</Label>
                <TextContainer text={key.key} />
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
