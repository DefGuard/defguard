import clipboard from 'clipboardy';
import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import QRCode from 'react-qr-code';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import {
  ActionButton,
  ActionButtonVariant,
} from '../../../../../../../shared/components/layout/ActionButton/ActionButton';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/components/layout/Button/Button';
import { ExpandableCard } from '../../../../../../../shared/components/layout/ExpandableCard/ExpandableCard';
import { Input } from '../../../../../../../shared/components/layout/Input/Input';
import LoaderSpinner from '../../../../../../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../../shared/components/layout/MessageBox/MessageBox';
import {
  Select,
  SelectOption,
  SelectSizeVariant,
} from '../../../../../../../shared/components/layout/Select/Select';
import { useModalStore } from '../../../../../../../shared/hooks/store/useModalStore';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { AddDeviceConfig } from '../../../../../../../shared/types';
import { downloadWGConfig } from '../../../../../../../shared/utils/downloadWGConfig';

export const ConfigStep = () => {
  const { LL, locale } = useI18nContext();
  const configsData = useModalStore((state) => state.userDeviceModal.configs);
  const deviceName = useModalStore((state) => state.userDeviceModal.deviceName);
  const nextStep = useModalStore((state) => state.userDeviceModal.nextStep);
  const toaster = useToaster();
  const [selectedConfig, setSelectedConfig] = useState<
    SelectOption<AddDeviceConfig> | undefined
  >();

  const expandableCardActions = useMemo(() => {
    return [
      <ActionButton variant={ActionButtonVariant.QRCODE} key={1} forcedActive={true} />,
      <ActionButton
        variant={ActionButtonVariant.COPY}
        key={2}
        onClick={() => {
          if (selectedConfig) {
            clipboard
              .write(selectedConfig.value.config)
              .then(() => {
                toaster.success(
                  LL.modals.addDevice.web.steps.config.messages.copyConfig()
                );
              })
              .catch(() => {
                toaster.error(LL.messages.clipboardError());
              });
          }
        }}
      />,
      <ActionButton
        variant={ActionButtonVariant.DOWNLOAD}
        key={3}
        onClick={() => {
          if (selectedConfig) {
            downloadWGConfig(
              selectedConfig.value.config,
              `${deviceName
                ?.toLowerCase()
                .replace(' ', '')}-${selectedConfig.value.network_name
                .toLowerCase()
                .replace(' ', '')}.conf`
            );
          }
        }}
      />,
    ];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [configsData, deviceName, toaster, locale]);

  const getSelectOptions = useMemo((): SelectOption<AddDeviceConfig>[] => {
    if (configsData) {
      return configsData.map((c) => configToSelectOption(c));
    }
    return [];
  }, [configsData]);

  useEffect(() => {
    if (configsData && configsData.length && isUndefined(selectedConfig)) {
      setSelectedConfig(configToSelectOption(configsData[0]));
    }
  }, [configsData, selectedConfig]);

  return (
    <>
      <MessageBox type={MessageBoxType.WARNING}>
        {parse(LL.modals.addDevice.web.steps.config.warningMessage())}
      </MessageBox>
      <Input
        outerLabel={LL.modals.addDevice.web.steps.config.inputNameLabel()}
        value={deviceName ?? ''}
        onChange={() => {
          return;
        }}
        disabled={true}
      />
      <div className="info">
        <p>{LL.modals.addDevice.web.steps.config.qrInfo()}</p>
      </div>
      {configsData && configsData.length > 0 && (
        <ExpandableCard
          title={LL.modals.addDevice.web.steps.config.qrCardTitle()}
          actions={expandableCardActions}
          topExtras={
            <Select
              selected={selectedConfig}
              options={getSelectOptions}
              onChange={(o) => {
                if (!Array.isArray(o)) {
                  setSelectedConfig(o);
                }
              }}
              multi={false}
              searchable={false}
              sizeVariant={SelectSizeVariant.STANDARD}
            />
          }
          expanded
        >
          {selectedConfig && <QRCode value={selectedConfig.value.config} size={250} />}
          {isUndefined(selectedConfig) && <LoaderSpinner size={250} />}
        </ExpandableCard>
      )}
      <div className="controls">
        <Button
          text={LL.form.close()}
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => nextStep()}
        />
      </div>
    </>
  );
};

const configToSelectOption = (
  configData: AddDeviceConfig
): SelectOption<AddDeviceConfig> => ({
  value: configData,
  label: configData.network_name,
  key: configData.network_id,
});
