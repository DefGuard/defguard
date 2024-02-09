import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { Control, useController } from 'react-hook-form';

import { SelectRow } from '../../../../../../../shared/defguard-ui/components/Layout/SelectRow/SelectRow';
import { User } from '../../../../../../../shared/types';
import { titleCase } from '../../../../../../../shared/utils/titleCase';
import { ModifyGroupFormFields } from '../../AddGroupModal';

type Props = {
  control: Control<ModifyGroupFormFields>;
  user: User;
};

export const UserSelect = ({ control, user }: Props) => {
  const {
    field: { value, onChange },
  } = useController({
    control: control,
    name: 'members',
  });

  const selected = useMemo(
    () => !isUndefined(value.find((s) => s === user.username)),
    [user.username, value],
  );

  return (
    <SelectRow
      selected={selected}
      onClick={() => {
        if (selected) {
          onChange(value.filter((s) => s !== user.username));
        } else {
          onChange([...value, user.username]);
        }
      }}
    >
      <p>{titleCase(`${user.first_name} ${user.last_name}`)}</p>
      <p>{user.username}</p>
    </SelectRow>
  );
};
