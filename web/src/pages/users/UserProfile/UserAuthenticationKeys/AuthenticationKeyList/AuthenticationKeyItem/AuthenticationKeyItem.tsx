import classNames from 'classnames';
import saveAs from 'file-saver';
import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { DeviceAvatar } from '../../../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { Label } from '../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { TextContainer } from '../../../../../../shared/defguard-ui/components/Layout/TextContainer/TextContainer';
import { useUserProfileStore } from '../../../../../../shared/hooks/store/useUserProfileStore';
import { AuthenticationKey } from '../../../../../../shared/types';
import { useRenameAuthenticationKeyModal } from '../../../../shared/modals/RenameAuthenticationKeyModal/useRenameAuthenticationKeyModal';
import { useDeleteAuthenticationKeyModal } from '../../DeleteAuthenticationKeyModal/useDeleteAuthenticationKeyModal';

type Props = {
  keyData: AuthenticationKey;
};

export const AuthenticationKeyItem = ({ keyData: key }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.userPage.authenticationKeys.keysList;
  const [hovered, setHovered] = useState(false);
  const username = useUserProfileStore((s) => s.userProfile?.user.username);
  const openRenameModal = useRenameAuthenticationKeyModal((s) => s.open);
  const openDeleteModal = useDeleteAuthenticationKeyModal((s) => s.open);

  const cn = useMemo(
    () =>
      classNames('authentication-key-item', {
        active: hovered,
      }),
    [hovered],
  );

  const downloadKey = useCallback(() => {
    const data = new Blob([key.key], { type: 'text/plain;charset=utf-8' });
    saveAs(
      data,
      `${key.name.replace(' ', '').toLocaleLowerCase()}_${key.key_type.valueOf().toLowerCase()}.pub`,
    );
  }, [key.key, key.key_type, key.name]);

  return (
    <div
      className={cn}
      onMouseOver={() => setHovered(true)}
      onMouseOut={() => setHovered(false)}
    >
      <header className="item-content">
        <div className="top">
          <DeviceAvatar deviceId={key.id} active={false} />
          <TextContainer text={key.name} />
        </div>
        <div className="bottom">
          <Label>{localLL.common.key()}</Label>
          <TextContainer text={key.key} />
        </div>
        <div className="controls">
          <EditButton>
            <EditButtonOption
              text={`${localLL.common.download()} ${localLL.common.key()}`}
              onClick={() => downloadKey()}
            />
            <EditButtonOption
              text={`${localLL.common.rename()} ${localLL.common.key()}`}
              onClick={() => {
                if (username) {
                  openRenameModal({
                    id: key.id,
                    key_type: key.key_type,
                    name: key.name,
                    username,
                  });
                }
              }}
            />
            <EditButtonOption
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              text={`${localLL.common.delete()} ${localLL.common.key()}`}
              onClick={() => {
                if (username) {
                  openDeleteModal({
                    id: key.id,
                    name: key.name,
                    type: key.key_type,
                    username,
                  });
                }
              }}
            />
          </EditButton>
        </div>
      </header>
    </div>
  );
};
