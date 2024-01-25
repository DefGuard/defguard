import './style.scss';

import classNames from 'classnames';
import saveAs from 'file-saver';
import { TargetAndTransition } from 'framer-motion';
import { useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
// import SvgIconCollapse from '../../../../../shared/components/svg/IconCollapse';
// import SvgIconExpand from '../../../../../shared/components/svg/IconExpand';
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
  authentication_key: AuthenticationKey;
}

export const AuthenticationKeyCard = ({ authentication_key }: Props) => {
  const [hovered, setHovered] = useState(false);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [expanded, setExpanded] = useState(false);

  const { LL } = useI18nContext();
  // const toaster = useToaster();
  // const queryClient = useQueryClient();
  const { writeToClipboard } = useClipboard();

  const cn = useMemo(
    () =>
      classNames('device-card', {
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
          <DeviceAvatar deviceId={Number(authentication_key.id)} active={false} />
          <h3 data-testid="device-name">{authentication_key.name}</h3>
        </header>
        <div className="section-content">
          <div>
            <Label>{LL.userPage.authenticationKeys.keyCard.keyLabel()}</Label>
            <p
              data-testid="authentication-key-value"
              className="authentication-key-value"
            >
              {authentication_key.key}
            </p>
          </div>
        </div>
      </section>

      <div className="locations">
        {/* {orderedLocations.map((n) => (
          <DeviceLocation key={n.network_id} network_info={n} />
        ))} */}
      </div>
      <div className="card-controls">
        <EditButton visible={true}>
          <EditButtonOption
            text={LL.userPage.authenticationKeys.keyCard.copyToClipboard()}
            onClick={() => {
              writeToClipboard(authentication_key.key);
            }}
          />
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.STANDARD}
            text={LL.userPage.authenticationKeys.keyCard.downloadKey()}
            onClick={() => {
              const blob = new Blob([authentication_key.key], {
                type: 'text/plain;charset=utf-8',
              });
              saveAs(
                blob,
                `${authentication_key.name.replace(' ', '_').toLocaleLowerCase()}.txt`,
              );
            }}
          />
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text={LL.userPage.authenticationKeys.keyCard.deleteKey()}
            onClick={() => {
              openModal({ visible: true, authenticationKey: authentication_key });
            }}
          />
        </EditButton>
        {/* <ExpandButton
          expanded={expanded}
          onClick={() => setExpanded((state) => !state)}
        /> */}
      </div>
    </Card>
  );
};

// type ExpandButtonProps = {
//   expanded: boolean;
//   onClick: () => void;
// };

// const ExpandButton = ({ expanded, onClick }: ExpandButtonProps) => {
//   return (
//     <button className="device-card-expand" onClick={onClick}>
//       {expanded ? <SvgIconCollapse /> : <SvgIconExpand />}
//     </button>
//   );
// };
