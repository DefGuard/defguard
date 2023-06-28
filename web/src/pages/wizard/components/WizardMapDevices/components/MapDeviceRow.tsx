import { useEffect, useMemo, useState } from 'react';
import { Control, useController } from 'react-hook-form';

import { RowBox } from '../../../../../shared/components/layout/RowBox/RowBox';
import {
  Select,
  SelectOption,
  SelectSizeVariant,
  SelectStyleVariant,
} from '../../../../../shared/components/layout/Select/Select';
import { WizardMapFormValues } from '../WizardMapDevices';

type Props = {
  options: SelectOption<number>[];
  control: Control<WizardMapFormValues>;
  index: number;
};

export const MapDeviceRow = ({ options, control, index }: Props) => {
  const [search, setSearch] = useState<string | undefined>();

  const nameController = useController({
    control,
    name: `devices.${index}.name`,
  });

  const userController = useController({
    control,
    name: `devices.${index}.user_id`,
  });

  const ipController = useController({
    control,
    name: `devices.${index}.wireguard_ip`,
  });

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
    () => options.find((u) => u.value === userController.field.value),
    [options, userController.field.value]
  );

  useEffect(() => {
    console.log(userController.field.value);
    console.log(nameController.field.value);
    console.log(ipController.field.value);
  }, [ipController.field.value, nameController.field.value, userController.field.value]);

  return (
    <RowBox className="device">
      <span className="name">{nameController.field.value}</span>
      <span className="ip">{ipController.field.value}</span>
      <Select<number>
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
            userController.field.onChange(res.value);
          }
        }}
      />
    </RowBox>
  );
};
