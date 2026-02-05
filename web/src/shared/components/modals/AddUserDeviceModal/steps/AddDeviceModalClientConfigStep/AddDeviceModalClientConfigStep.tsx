import { useMemo, useState } from 'react';
import { m } from '../../../../../../paraglide/messages';
import { SizedBox } from '../../../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../defguard-ui/types';
import { useAddUserDeviceModal } from '../../store/useAddUserDeviceModal';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { QRCodeCanvas } from 'qrcode.react';
import { titleCase } from 'text-case';
import { externalLink } from '../../../../../constants';
import { Button } from '../../../../../defguard-ui/components/Button/Button';
import { CopyField } from '../../../../../defguard-ui/components/CopyField/CopyField';
import { Divider } from '../../../../../defguard-ui/components/Divider/Divider';
import { Fold } from '../../../../../defguard-ui/components/Fold/Fold';
import { FoldButton } from '../../../../../defguard-ui/components/FoldButton/FoldButton';
import { IconButton } from '../../../../../defguard-ui/components/IconButton/IconButton';
import { IconButtonMenu } from '../../../../../defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../../../defguard-ui/components/Menu/types';
import { clientArtifactsQueryOptions } from '../../../../../query';
import { openClientLink } from '../../../../../utils/openVirtualLink';
import { ContainerWithIcon } from '../../../../ContainerWithIcon/ContainerWithIcon';
import { Controls } from '../../../../Controls/Controls';

export const AddDeviceModalClientConfigStep = () => {
  const enrollment = useAddUserDeviceModal((s) => s.enrollment);
  const [manualOpen, setManualOpen] = useState(false);
  const { data: clientLinks } = useQuery(clientArtifactsQueryOptions);

  const deepLink = useMemo(() => {
    if (!enrollment) return null;
    return `defguard://addinstance?token=${enrollment.token}&url=${enrollment.url}`;
  }, [enrollment]);

  const qrData = useMemo(() => {
    if (!enrollment) return null;
    return btoa(
      JSON.stringify({
        url: enrollment.url,
        token: enrollment.token,
      }),
    );
  }, [enrollment]);

  const appleMenu = useMemo(
    (): MenuItemsGroup[] => [
      {
        header: {
          text: 'Apple Hardware',
        },
        items: [
          {
            icon: 'apple',
            text: 'Intel',
            onClick: () => openClientLink(clientLinks?.macos_amd64),
          },
          {
            icon: 'apple',
            text: 'ARM',
            onClick: () => openClientLink(clientLinks?.macos_arm64),
          },
        ],
      },
    ],
    [clientLinks],
  );

  const linuxMenu = useMemo(() => {
    const res: MenuItemsGroup[] = [
      {
        header: {
          text: `${titleCase(m.misc_for())} Linux`,
        },
        items: [
          {
            icon: 'ubuntu',
            text: 'Ubuntu 24.04 ARM',
            onClick: () => openClientLink(clientLinks?.deb_arm64),
          },
          {
            icon: 'ubuntu',
            text: 'Ubuntu 24.04 AMD64',
            onClick: () => openClientLink(clientLinks?.deb_amd64),
          },
        ],
      },
      {
        items: [
          {
            icon: 'debian',
            text: 'Ubuntu 22.04 / Debian 12&13 ARM',
            onClick: () => openClientLink(clientLinks?.deb_legacy_arm64),
          },
          {
            icon: 'debian',
            text: 'Ubuntu 22.04 / Debian 12&13 AMD64',
            onClick: () => openClientLink(clientLinks?.deb_legacy_amd64),
          },
        ],
      },
      {
        items: [
          {
            icon: 'linux',
            text: 'RPM ARM',
            onClick: () => openClientLink(clientLinks?.rpm_arm64),
          },
          {
            icon: 'linux',
            text: 'RPM AMD64',
            onClick: () => openClientLink(clientLinks?.rpm_amd64),
          },
        ],
      },
      {
        items: [
          {
            icon: 'arch-linux',
            text: 'Arch Linux',
            onClick: () => openClientLink(externalLink.client.desktop.linux.arch),
          },
        ],
      },
    ];
    return res;
  }, [clientLinks]);

  if (!deepLink || !qrData || !enrollment) return null;

  return (
    <div id="add-device-client-step">
      <header>
        <p>{m.modal_add_user_device_client_title()}</p>
        <p>{m.modal_add_user_device_client_subtitle()}</p>
      </header>
      <SizedBox height={ThemeSpacing.Xl2} />
      <ContainerWithIcon icon="desktop" id="setup-desktop">
        <header>
          <h5>{m.modal_add_user_device_client_desktop_title()}</h5>
          <p>{m.modal_add_user_device_client_desktop_automatic()}</p>
          <p>
            {m.modal_add_user_device_client_desktop_automatic_explain_1()}{' '}
            <span>{m.modal_add_user_device_client_desktop_automatic_explain_2()}</span>
          </p>
        </header>
        <div className="buttons">
          <a href={deepLink} target="_blank">
            <Button
              text={m.modal_add_user_device_client_desktop_one_click()}
              variant="primary"
              iconRight="open-in-new-window"
            />
          </a>
          <div className="download">
            <p>{m.modal_add_user_device_client_desktop_download()}</p>
            <a
              href={clientLinks?.windows_amd64 ?? externalLink.defguard.download}
              target="_blank"
              rel="noopener noreferrer"
            >
              <IconButton icon="windows" />
            </a>
            <IconButtonMenu icon="apple" menuItems={appleMenu} />
            <IconButtonMenu icon="linux" menuItems={linuxMenu} />
          </div>
        </div>
        <Divider orientation="horizontal" />
        <Fold open={manualOpen} contentClassName="manual">
          <p className="title">{m.modal_add_user_device_client_desktop_manual_title()}</p>
          <p className="subtitle">
            {m.modal_add_user_device_client_desktop_manual_subtitle()}
          </p>
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="form-col-2">
            <CopyField
              text={enrollment?.url}
              copyTooltip={m.controls_copy_clipboard()}
              label={m.form_label_url()}
            />
            <CopyField
              copyTooltip={m.controls_copy_clipboard()}
              text={enrollment.token}
              label={m.form_label_token()}
            />
          </div>
          <SizedBox height={ThemeSpacing.Xl2} />
        </Fold>
        <FoldButton
          textClose={m.modal_add_user_device_client_fold_open()}
          textOpen={m.modal_add_user_device_client_fold_closed()}
          open={manualOpen}
          onChange={setManualOpen}
        />
      </ContainerWithIcon>
      <SizedBox height={ThemeSpacing.Md} />
      <ContainerWithIcon id="setup-mobile" icon="phone">
        <header>
          <h5>{m.modal_add_user_device_client_mobile_title()}</h5>
          <p>{m.modal_add_user_device_client_mobile_subtitle()}</p>
        </header>
        <div className="bottom">
          <div className="qr">
            <QRCodeCanvas value={qrData} size={200} />
          </div>
          <div className="download">
            <p>{m.modal_add_user_device_client_mobile_get_mobile()}</p>
            <div className="links">
              <a
                href={externalLink.client.mobile.google}
                target="_blank"
                rel="noopener noreferrer"
              >
                <Button variant="outlined" iconLeft="android" text={'Google Play'} />
              </a>
              <a
                href={externalLink.client.mobile.apple}
                target="_blank"
                rel="noopener noreferrer"
              >
                <Button variant="outlined" iconLeft="apple" text={'Apple Store'} />
              </a>
            </div>
          </div>
        </div>
      </ContainerWithIcon>
      <Controls>
        <div className="left">
          <p>{`Once your Defguard client is configured, you can close this window.`}</p>
        </div>
        <div className="right">
          <Button
            text={m.controls_close()}
            onClick={() => {
              useAddUserDeviceModal.getState().close();
            }}
          />
        </div>
      </Controls>
    </div>
  );
};
