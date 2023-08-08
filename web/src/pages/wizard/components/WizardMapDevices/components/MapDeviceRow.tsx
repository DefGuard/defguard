import { TargetAndTransition } from 'framer-motion';
import { useMemo, useState } from 'react';
import { Control, useController } from 'react-hook-form';

import { ColorsRGB } from '../../../../../shared/constants';
import { RowBox } from '../../../../../shared/defguard-ui/components/Layout/RowBox/RowBox';
import { Select } from '../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
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
          (o.meta as string).includes(search.toLowerCase()),
      );
    }
    return options;
  }, [options, search]);

  const getSelected = useMemo(
    () => options.find((u) => u.value === userController.field.value),
    [options, userController.field.value],
  );

  const hasErrors = useMemo(() => {
    return nameController.fieldState.invalid || userController.fieldState.invalid;
  }, [nameController.fieldState.invalid, userController.fieldState.invalid]);

  const getAnimate = useMemo(() => {
    const res: TargetAndTransition = {
      borderColor: ColorsRGB.GrayBorder,
    };
    if (hasErrors) {
      res.borderColor = ColorsRGB.Error;
    }
    return res;
  }, [hasErrors]);

  return (
    <RowBox className="device" customAnimate={getAnimate}>
      <input className="name" type="text" {...nameController.field} />
      <span className="ip">{ipController.field.value}</span>
      <Select<number>
        data-testid={`user-select-${index}`}
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
