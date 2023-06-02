import { useMemo, useState } from 'react';

import { RowBox } from '../../../../../shared/components/layout/RowBox/RowBox';
import {
  Select,
  SelectOption,
  SelectSizeVariant,
  SelectStyleVariant,
} from '../../../../../shared/components/layout/Select/Select';
import { ImportedDevice } from '../../../../../shared/types';
import { useWizardStore } from '../../../hooks/useWizardStore';

type Props = {
  device: ImportedDevice;
  options: SelectOption<number>[];
  testId?: string;
};
export const MapDeviceRow = ({ options, device, testId }: Props) => {
  const mapDevice = useWizardStore((state) => state.mapDevice);
  const [search, setSearch] = useState<string | undefined>();
  const getOptions = useMemo(() => {
    if (search && search.length) {
      return options.filter(
        (o) =>
          o.label.toLocaleLowerCase().includes(search.toLocaleLowerCase()) ||
          (o.meta as string).includes(search.toLowerCase())
      );
    }
    return options;
  }, [options, search]);
  const getSelected = useMemo(
    () => options.find((u) => u.value === device.user_id),
    [device.user_id, options]
  );
  return (
    <RowBox className="device" data-testid={testId}>
      <span className="name">{device.name}</span>
      <span className="ip">{device.wireguard_ip}</span>
      <Select
        searchable
        styleVariant={SelectStyleVariant.LIGHT}
        sizeVariant={SelectSizeVariant.SMALL}
        selected={getSelected}
        options={getOptions}
        placeholder="Choose a user"
        onSearch={setSearch}
        searchDebounce={50}
        onChange={(res) => {
          if (!Array.isArray(res) && res) {
            mapDevice(device.wireguard_ip, res.value);
          }
        }}
      />
    </RowBox>
  );
};
