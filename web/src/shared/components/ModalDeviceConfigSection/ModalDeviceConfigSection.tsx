import { QRCodeCanvas } from 'qrcode.react';
import type { AddDeviceResponse, AddDeviceResponseConfig } from '../../api/types';
import './style.scss';
import { ZipArchive } from '@shortercode/webzip';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Button } from '../../defguard-ui/components/Button/Button';
import { ButtonMenu } from '../../defguard-ui/components/ButtonMenu/MenuButton';
import { EvenSplit } from '../../defguard-ui/components/EvenSplit/EvenSplit';
import type { MenuItemsGroup } from '../../defguard-ui/components/Menu/types';
import { Select } from '../../defguard-ui/components/Select/Select';
import type { SelectOption } from '../../defguard-ui/components/Select/types';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { useClipboard } from '../../defguard-ui/hooks/useClipboard';
import { ThemeSpacing } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { downloadFile, downloadText } from '../../utils/download';

const configToOption = (
  item: AddDeviceResponseConfig,
): SelectOption<AddDeviceResponseConfig> => ({
  key: item.network_id,
  label: item.network_name,
  value: item,
});

const configToFilename = (item: AddDeviceResponseConfig): string =>
  `${item.network_name.toLowerCase().replaceAll(' ', '-')}.txt`;

type Props = { data: AddDeviceResponse; privateKey?: string };

export const ModalDeviceConfigSection = ({ data: response, privateKey }: Props) => {
  const publicKey = response.device.wireguard_pubkey;
  const { writeToClipboard } = useClipboard();
  const [selectedOption, setSelected] = useState<SelectOption<AddDeviceResponseConfig>>(
    configToOption(response.configs[0]),
  );

  const qrConfig = useMemo(() => {
    const config = selectedOption.value.config;
    return config.replace('YOUR_PRIVATE_KEY', privateKey ?? publicKey);
  }, [selectedOption, privateKey, publicKey]);

  const clipboardConfig = useMemo(() => {
    const config = selectedOption.value.config;
    if (privateKey) {
      return config.replace('YOUR_PRIVATE_KEY', privateKey);
    }
    return config;
  }, [selectedOption, privateKey]);

  const handleDownloadSelected = useCallback(() => {
    downloadText(clipboardConfig, 'conf');
  }, [clipboardConfig]);

  const handleDownloadAll = useCallback(async () => {
    if (!response) return;
    let data: AddDeviceResponseConfig[] = [];
    if (isPresent(privateKey)) {
      data = response.configs.map((c) => ({
        ...c,
        config: c.config.replace('YOUR_PRIVATE_KEY', privateKey as string),
      }));
    } else {
      data = response.configs;
    }
    const zip = new ZipArchive();
    for (const item of data) {
      await zip.set(configToFilename(item), item.config);
    }
    const blob = zip.to_blob();
    downloadFile(blob, 'locations', 'zip');
  }, [response, privateKey]);

  const downloadMenu = useMemo(
    (): MenuItemsGroup[] => [
      {
        items: [
          {
            text: m.modal_add_user_device_manual_download_actions_download_all(),
            onClick: handleDownloadAll,
          },
          {
            text: m.modal_add_user_device_manual_download_actions_download_one(),
            onClick: handleDownloadSelected,
          },
        ],
      },
    ],
    [handleDownloadAll, handleDownloadSelected],
  );

  const selectOptions = useMemo(
    () =>
      response.configs.map(
        (item): SelectOption<AddDeviceResponseConfig> => ({
          key: item.network_id,
          label: item.network_name,
          value: item,
        }),
      ),
    [response.configs],
  );
  return (
    <div className="modal-device-config-section">
      <QRCodeCanvas size={160} value={qrConfig} />
      <div className="right">
        <Select
          label={m.modal_add_user_device_manual_download_location_label()}
          options={selectOptions}
          onChange={setSelected}
          value={selectedOption}
        />
        <SizedBox height={ThemeSpacing.Xl2} />
        <p>{m.modal_add_user_device_manual_download_explain()}</p>
        <EvenSplit spacing={ThemeSpacing.Md} parts={2}>
          <ButtonMenu
            variant="outlined"
            iconLeft="download"
            text={m.modal_add_user_device_manual_download_actions_download()}
            menuItems={downloadMenu}
          />
          <Button
            text={m.controls_copy_clipboard()}
            variant="outlined"
            iconLeft="copy"
            onClick={() => {
              void writeToClipboard(clipboardConfig);
            }}
          />
        </EvenSplit>
      </div>
    </div>
  );
};
