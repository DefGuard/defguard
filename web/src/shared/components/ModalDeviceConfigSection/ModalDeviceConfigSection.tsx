import type { AddDeviceResponse, AddDeviceResponseConfig } from '../../api/types';
import './style.scss';
import { ZipArchive } from '@shortercode/webzip';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { ButtonMenu } from '../../defguard-ui/components/ButtonMenu/MenuButton';
import type { MenuItemsGroup } from '../../defguard-ui/components/Menu/types';
import { QrCard } from '../../defguard-ui/components/QrCard/QrCard';
import { Select } from '../../defguard-ui/components/Select/Select';
import type { SelectOption } from '../../defguard-ui/components/Select/types';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { downloadFile, downloadText } from '../../utils/download';
import { CopyButton } from '../CopyButton/CopyButton';

const configToOption = (
  item: AddDeviceResponseConfig,
): SelectOption<AddDeviceResponseConfig> => ({
  key: item.network_id,
  label: item.network_name,
  value: item,
});

const configToLocationName = (item: AddDeviceResponseConfig): string =>
  item.network_name.toLowerCase().replaceAll(' ', '-');

const configToFilename = (item: AddDeviceResponseConfig): string =>
  `${configToLocationName(item)}.conf`;

type Props = { data: AddDeviceResponse; privateKey?: string };

export const ModalDeviceConfigSection = ({ data: response, privateKey }: Props) => {
  const publicKey = response.device.wireguard_pubkey;
  const selectOptions = useMemo(
    () =>
      response.configs
        // A config is downloadable only when the location needs neither MFA nor a posture
        // check. `mfa_enabled` is the authoritative flag; the legacy `location_mfa_mode` is
        // derived and absent for flows with no legacy equivalent, so it cannot be used here.
        .filter((item) => !item.mfa_enabled && !item.posture_check_required)
        .map((item): SelectOption<AddDeviceResponseConfig> => configToOption(item)),
    [response.configs],
  );
  const [selectedOption, setSelected] =
    useState<SelectOption<AddDeviceResponseConfig> | null>(selectOptions[0] ?? null);
  const hasConfigs = isPresent(selectedOption);

  const qrConfig = useMemo(() => {
    if (!selectedOption) return '';
    const config = selectedOption.value.config;
    return config.replace('YOUR_PRIVATE_KEY', privateKey ?? publicKey);
  }, [selectedOption, privateKey, publicKey]);

  const clipboardConfig = useMemo(() => {
    if (!selectedOption) return '';
    const config = selectedOption.value.config;
    if (privateKey) {
      return config.replace('YOUR_PRIVATE_KEY', privateKey);
    }
    return config;
  }, [selectedOption, privateKey]);

  const handleDownloadSelected = useCallback(() => {
    if (!selectedOption) return;
    downloadText(clipboardConfig, configToLocationName(selectedOption.value), 'conf');
  }, [clipboardConfig, selectedOption]);

  const handleDownloadAll = useCallback(async () => {
    if (!response) return;
    const nonMfaConfigs = response.configs.filter(
      (c) => !c.mfa_enabled && !c.posture_check_required,
    );
    let data: AddDeviceResponseConfig[] = [];
    if (isPresent(privateKey)) {
      data = nonMfaConfigs.map((c) => ({
        ...c,
        config: c.config.replace('YOUR_PRIVATE_KEY', privateKey as string),
      }));
    } else {
      data = nonMfaConfigs;
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

  return (
    <div className="modal-device-config-section">
      {hasConfigs && <QrCard value={qrConfig} />}
      <div className="right">
        {selectedOption && (
          <Select
            label={m.modal_add_user_device_manual_download_location_label()}
            helper={m.modal_add_user_device_manual_download_location_helper()}
            options={selectOptions}
            onChange={setSelected}
            value={selectedOption}
            disabled={!hasConfigs}
          />
        )}
        <SizedBox height={ThemeSpacing.Xl2} />
        <p>{m.modal_add_user_device_manual_download_explain()}</p>
        {hasConfigs && (
          <div className="actions">
            <ButtonMenu
              variant="outlined"
              iconLeft="download"
              text={m.modal_add_user_device_manual_download_actions_download()}
              menuItems={downloadMenu}
            />
            <CopyButton value={clipboardConfig} />
          </div>
        )}
      </div>
    </div>
  );
};
