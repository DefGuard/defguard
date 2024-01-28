import './style.scss';

import classNames from 'classnames';
import saveAs from 'file-saver';
import { TargetAndTransition } from 'framer-motion';
import { useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ColorsRGB } from '../../../../../shared/constants';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { DeviceAvatar } from '../../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { useClipboard } from '../../../../../shared/hooks/useClipboard';
import { AuthenticationKey } from '../../../../../shared/types';
import { useDeleteAuthenticationKeyModal } from '../../../shared/modals/DeleteAuthenticationKeyModal/useDeleteAuthenticationKeyModal';

interface Props {
  authenticationKey: AuthenticationKey;
}

export const AuthenticationKeyCard = ({ authenticationKey }: Props) => {
  const [hovered, setHovered] = useState(false);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [expanded, _setExpanded] = useState(false);

  const { LL } = useI18nContext();
  const { writeToClipboard } = useClipboard();

  const cn = useMemo(
    () =>
      classNames('authentication-key-card', {
        expanded,
      }),
    [expanded],
  );

  const openModal = useDeleteAuthenticationKeyModal((state) => state.open, shallow);

  const getContainerAnimate = useMemo((): TargetAndTransition => {
    const res: TargetAndTransition = {
      borderColor: ColorsRGB.White,
    };
    if (expanded || hovered) {
      res.borderColor = ColorsRGB.GrayBorder;
    }
    return res;
  }, [expanded, hovered]);

  return (
    <Card
      className={cn}
      initial={false}
      animate={getContainerAnimate}
      onMouseOver={() => setHovered(true)}
      onMouseOut={() => setHovered(false)}
    >
      <section className="main-info">
        <header>
          <DeviceAvatar deviceId={Number(authenticationKey.id)} active={false} />
          <h3 data-testid="authentication-key-name">{authenticationKey.name}</h3>
        </header>
        <div className="section-content">
          <div className="authentication-key-value-container">
            <Label>{LL.userPage.authenticationKeys.keyCard.keyLabel()}</Label>
            <p
              data-testid="card-authentication-key-value"
              className="authentication-key-value"
            >
              {authenticationKey.key}
            </p>
          </div>
        </div>
      </section>

      <div className="locations"></div>
      <div className="card-controls">
        <EditButton visible data-testid="authentication-key-settings-button">
          <EditButtonOption
            text={LL.userPage.authenticationKeys.keyCard.copyToClipboard()}
            onClick={() => {
              writeToClipboard(authenticationKey.key);
            }}
          />
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.STANDARD}
            text={LL.userPage.authenticationKeys.keyCard.downloadKey()}
            onClick={() => {
              const blob = new Blob([authenticationKey.key], {
                type: 'text/plain;charset=utf-8',
              });
              saveAs(
                blob,
                `${authenticationKey.name.replace(' ', '_').toLocaleLowerCase()}.txt`,
              );
            }}
          />
          <EditButtonOption
            data-testid="authentication-key-settings-delete"
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text={LL.userPage.authenticationKeys.keyCard.deleteKey()}
            onClick={() => {
              openModal({ visible: true, authenticationKey });
            }}
          />
        </EditButton>
      </div>
    </Card>
  );
};
